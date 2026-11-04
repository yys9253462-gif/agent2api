package com.agent2api.mobile;

import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.provider.Settings;
import androidx.core.content.FileProvider;
import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;
import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.security.MessageDigest;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicBoolean;

/**
 * 应用内更新：下载 APK 到应用私有缓存目录，再交给系统安装器。
 *
 * ── 为什么自己写而不用 @capacitor/filesystem ──────────────────────
 * 那个插件没有「下载」能力（只有读写），而 `CapacitorHttp` 会把整个响应读进
 * 内存 —— 几十 MB 的安装包会直接把 WebView 撑爆。这里用 HttpURLConnection
 * 边读边写，内存占用只有缓冲区大小。
 *
 * ── 为什么放 cacheDir 而不是外部存储 ────────────────────────────
 * 不需要任何存储权限；FileProvider 通过 `file_paths.xml` 的 `<cache-path>`
 * 已经把这个目录纳管（见 `res/xml/file_paths.xml`）。
 *
 * ── 安装权限 ────────────────────────────────────────────────────
 * Android 8.0 起「安装未知来源应用」是按应用授权的。没授权时 `startActivity`
 * 会抛 ActivityNotFoundException 或静默失败，所以先查
 * `canRequestPackageInstalls()`；没有就返回 `needsPermission: true`，
 * 由界面引导用户去系统设置页打开（见 `openInstallPermissionSettings`）。
 * 不在这里直接跳设置页 —— 那样用户不知道发生了什么就被弹出应用。
 */
@CapacitorPlugin(name = "ApkUpdater")
public class ApkUpdaterPlugin extends Plugin {

    /** 下载线程：单线程足够（同一时刻只该有一个下载任务） */
    private final ExecutorService executor = Executors.newSingleThreadExecutor();

    /** 下载中的取消标志（`download` 与 `cancel` 之间共享） */
    private final AtomicBoolean cancelled = new AtomicBoolean(false);
    private volatile HttpURLConnection activeConnection;
    private static final long MAX_BYTES = 600L * 1024 * 1024;

    /** 目标文件：应用缓存目录下的固定名，重下时覆盖 */
    private File targetFile() {
        File dir = new File(getContext().getCacheDir(), "updates");
        if (!dir.exists()) {
            //noinspection ResultOfMethodCallIgnored
            dir.mkdirs();
        }
        return new File(dir, "agent2api-update.apk");
    }

    private File partialFile() {
        return new File(targetFile().getParentFile(), "agent2api-update.apk.part");
    }

    /**
     * 下载 APK。参数 `{ url, sha256, size }`，校验值由网关从官方 API 获取。
     * 手机自己出网，不经网关中转。
     *
     * 进度通过 `downloadProgress` 事件推送（`{received, total, percent}`），
     * 界面据此画进度条 —— 不用轮询，省一次往返。
     */
    @PluginMethod
    public void download(PluginCall call) {
        String url = call.getString("url", "");
        if (url == null || url.isEmpty()) {
            call.reject("缺少下载地址");
            return;
        }
        String expectedHash = call.getString("sha256", "");
        if (expectedHash == null) expectedHash = "";
        if (!expectedHash.isEmpty() && !expectedHash.matches("[0-9a-fA-F]{64}")) {
            call.reject("官方校验值格式无效");
            return;
        }
        final String hash = expectedHash;
        final long expectedSize = call.getData().optLong("size", 0);
        cancelled.set(false);

        executor.execute(() -> {
            FileOutputStream output = null;
            InputStream input = null;
            HttpURLConnection connection = null;
            try {
                File file = partialFile();
                if (file.exists() && !file.delete()) throw new Exception("无法清理上次下载的临时文件");
                File old = targetFile();
                if (old.exists() && !old.delete()) throw new Exception("无法清理上次下载的安装包");
                if (cancelled.get()) throw new Exception("已取消下载");

                connection = openFollowingRedirects(url, hash);
                int status = connection.getResponseCode();
                if (status != HttpURLConnection.HTTP_OK) {
                    throw new Exception("服务器返回 HTTP " + status);
                }

                long declared = connection.getContentLengthLong();
                long total = expectedSize > 0 ? expectedSize : declared;
                if (total > MAX_BYTES || declared > MAX_BYTES) throw new Exception("安装包超过体积上限");
                input = connection.getInputStream();
                output = new FileOutputStream(file);
                MessageDigest digest = MessageDigest.getInstance("SHA-256");
                byte[] buffer = new byte[64 * 1024];
                long received = 0;
                long lastEmit = 0;
                emitProgress(0, total);
                int read;
                while ((read = input.read(buffer)) != -1) {
                    if (cancelled.get()) throw new Exception("已取消下载");
                    received += read;
                    if (received > MAX_BYTES) throw new Exception("安装包超过体积上限");
                    output.write(buffer, 0, read);
                    digest.update(buffer, 0, read);
                    long now = System.currentTimeMillis();
                    if (now - lastEmit > 200) {
                        lastEmit = now;
                        emitProgress(received, total);
                    }
                }
                output.flush();
                output.close();
                output = null;
                input.close();
                input = null;
                if (cancelled.get()) throw new Exception("已取消下载");
                if (received == 0 || (declared > 0 && received != declared)
                    || (expectedSize > 0 && received != expectedSize)) {
                    throw new Exception("安装包不完整，已拒绝使用");
                }
                if (!hash.isEmpty()) {
                    StringBuilder actual = new StringBuilder();
                    for (byte value : digest.digest()) {
                        actual.append(String.format(java.util.Locale.ROOT, "%02x", value & 0xff));
                    }
                    if (!hash.equalsIgnoreCase(actual.toString())) {
                        throw new Exception("安装包 SHA-256 与 GitHub 官方校验值不一致，已拒绝使用");
                    }
                }
                File target = targetFile();
                if (!file.renameTo(target)) throw new Exception("无法保存校验后的安装包");
                emitProgress(received, total);
                JSObject result = new JSObject();
                result.put("path", target.getAbsolutePath());
                result.put("size", received);
                call.resolve(result);
            } catch (Exception error) {
                partialFile().delete();
                call.reject(cancelled.get() ? "已取消下载" : "下载失败：" + error.getMessage());
            } finally {
                try { if (input != null) input.close(); } catch (Exception ignored) { }
                try { if (output != null) output.close(); } catch (Exception ignored) { }
                if (connection != null) connection.disconnect();
                activeConnection = null;
                partialFile().delete();
            }
        });
    }

    /** 下载候选源逐跳校验域名，只有官方校验值存在时才接受第三方源。 */
    private boolean isGithubAsset(URL url) {
        String host = url.getHost().toLowerCase(java.util.Locale.ROOT);
        return "https".equals(url.getProtocol()) && (url.getPort() == -1 || url.getPort() == 443)
            && (host.endsWith(".githubusercontent.com")
                || ((host.equals("github.com") || host.equals("www.github.com"))
                    && url.getPath().contains("/releases/download/")));
    }

    private HttpURLConnection openFollowingRedirects(String startUrl, String hash) throws Exception {
        URL origin = new URL(startUrl);
        if (!"https".equals(origin.getProtocol()) || origin.getUserInfo() != null) {
            throw new Exception("安装包下载必须使用无凭证的 HTTPS 地址");
        }
        if (!isGithubAsset(origin) && hash.isEmpty()) {
            throw new Exception("加速下载缺少 GitHub 官方校验值");
        }
        String current = startUrl;
        for (int hop = 0; hop < 5; hop++) {
            if (cancelled.get()) throw new Exception("已取消下载");
            URL target = new URL(current);
            boolean sameOrigin = target.getHost().equalsIgnoreCase(origin.getHost())
                && target.getPort() == origin.getPort();
            if (!"https".equals(target.getProtocol()) || target.getUserInfo() != null
                || (!sameOrigin && !isGithubAsset(target))) {
                throw new Exception("下载源重定向到未允许的地址");
            }
            HttpURLConnection connection = (HttpURLConnection) target.openConnection();
            activeConnection = connection;
            connection.setRequestMethod("GET");
            connection.setConnectTimeout(8_000);
            connection.setReadTimeout(hop == 0 ? 8_000 : 30_000);
            connection.setInstanceFollowRedirects(false);
            // GitHub 对没有 User-Agent 的请求直接 403
            connection.setRequestProperty("User-Agent", "agent2api-mobile");
            connection.setRequestProperty("Accept", "application/octet-stream");

            int status = connection.getResponseCode();
            if (status == HttpURLConnection.HTTP_MOVED_PERM
                || status == HttpURLConnection.HTTP_MOVED_TEMP
                || status == HttpURLConnection.HTTP_SEE_OTHER
                || status == 307
                || status == 308) {
                String location = connection.getHeaderField("Location");
                connection.disconnect();
                if (location == null || location.isEmpty()) {
                    throw new Exception("重定向缺少 Location");
                }
                // 相对地址按当前 URL 解析（GitHub 目前给绝对地址，但别假定）
                current = new URL(new URL(current), location).toString();
                continue;
            }
            connection.setReadTimeout(30_000);
            return connection;
        }
        throw new Exception("重定向次数过多");
    }

    private void emitProgress(long received, long total) {
        JSObject data = new JSObject();
        data.put("received", received);
        data.put("total", total);
        data.put("percent", total > 0 ? (int) (received * 100 / total) : -1);
        notifyListeners("downloadProgress", data);
    }

    /** 取消正在进行的下载（已下好的包不动 —— 取消只针对「正在下」这个动作） */
    @PluginMethod
    public void cancel(PluginCall call) {
        cancelled.set(true);
        HttpURLConnection connection = activeConnection;
        if (connection != null) connection.disconnect();
        call.resolve();
    }

    /**
     * 当前是否允许安装未知来源应用。
     *
     * Android 8.0 以下没有这个概念（默认允许），直接返回 true —— 那些版本
     * 由系统安装界面自己提示。
     */
    @PluginMethod
    public void canInstall(PluginCall call) {
        JSObject result = new JSObject();
        boolean allowed = true;
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            allowed = getContext().getPackageManager().canRequestPackageInstalls();
        }
        result.put("allowed", allowed);
        call.resolve(result);
    }

    /** 跳到本应用的「安装未知应用」设置页（`canInstall` 为 false 时由界面调用） */
    @PluginMethod
    public void openInstallPermissionSettings(PluginCall call) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Intent intent = new Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES);
            intent.setData(Uri.parse("package:" + getContext().getPackageName()));
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            getContext().startActivity(intent);
        }
        call.resolve();
    }

    /**
     * 调起系统安装器。
     *
     * 用 FileProvider 生成 `content://` URI：Android 7.0 起直接传 `file://`
     * 会抛 FileUriExposedException。`FLAG_GRANT_READ_URI_PERMISSION` 把临时读
     * 权限授给安装器进程 —— 少了这个，安装器打不开文件、表现为「解析包出错」。
     */
    @PluginMethod
    public void install(PluginCall call) {
        File file = targetFile();
        if (!file.exists()) {
            call.reject("安装包不存在，请先下载");
            return;
        }
        try {
            Uri uri = FileProvider.getUriForFile(
                getContext(),
                getContext().getPackageName() + ".fileprovider",
                file
            );
            Intent intent = new Intent(Intent.ACTION_VIEW);
            intent.setDataAndType(uri, "application/vnd.android.package-archive");
            intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            getContext().startActivity(intent);
            call.resolve();
        } catch (Exception error) {
            call.reject("无法启动安装器：" + error.getMessage());
        }
    }

    /** 当前已下载的安装包信息（重进设置页时据此显示「已下载，待安装」） */
    @PluginMethod
    public void downloaded(PluginCall call) {
        File file = targetFile();
        JSObject result = new JSObject();
        result.put("exists", file.exists());
        result.put("size", file.exists() ? file.length() : 0);
        call.resolve(result);
    }

    @Override
    protected void handleOnDestroy() {
        cancelled.set(true);
        HttpURLConnection connection = activeConnection;
        if (connection != null) connection.disconnect();
        executor.shutdownNow();
        super.handleOnDestroy();
    }
}
