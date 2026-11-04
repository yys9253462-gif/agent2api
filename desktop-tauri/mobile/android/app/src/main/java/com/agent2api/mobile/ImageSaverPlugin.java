package com.agent2api.mobile;

import android.app.Activity;
import android.content.ContentValues;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.os.Environment;
import android.provider.MediaStore;
import android.util.Base64;
import androidx.activity.result.ActivityResult;
import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.ActivityCallback;
import com.getcapacitor.annotation.CapacitorPlugin;
import java.io.OutputStream;

@CapacitorPlugin(name = "ImageSaver")
public class ImageSaverPlugin extends Plugin {
    @PluginMethod
    public void save(PluginCall call) {
        String data = call.getString("data", "");
        String name = call.getString("name", "");
        String mime = call.getString("mime", "");
        if (data.isEmpty() || !name.matches("image-[a-f0-9]{8}\\.(png|jpg|webp)")
                || !(mime.equals("image/png") || mime.equals("image/jpeg") || mime.equals("image/webp"))
                || data.length() > 40 * 1024 * 1024) {
            call.reject("图片参数无效");
            return;
        }
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q) {
            Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT);
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            intent.setType(mime);
            intent.putExtra(Intent.EXTRA_TITLE, name);
            try { startActivityForResult(call, intent, "saveDocument"); }
            catch (Exception error) { call.reject("无法打开保存文件窗口：" + error.getMessage()); }
            return;
        }
        Uri uri = null;
        try {
            byte[] bytes = Base64.decode(data, Base64.DEFAULT);
            ContentValues values = new ContentValues();
            values.put(MediaStore.Images.Media.DISPLAY_NAME, name);
            values.put(MediaStore.Images.Media.MIME_TYPE, mime);
            values.put(MediaStore.Images.Media.RELATIVE_PATH, Environment.DIRECTORY_PICTURES + "/Agent2API");
            values.put(MediaStore.Images.Media.IS_PENDING, 1);
            uri = getContext().getContentResolver().insert(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, values);
            if (uri == null) throw new Exception("无法创建图片文件");
            try (OutputStream output = getContext().getContentResolver().openOutputStream(uri)) {
                if (output == null) throw new Exception("无法打开图片文件");
                output.write(bytes);
            }
            ContentValues complete = new ContentValues();
            complete.put(MediaStore.Images.Media.IS_PENDING, 0);
            getContext().getContentResolver().update(uri, complete, null, null);
            JSObject result = new JSObject();
            result.put("saved", true);
            result.put("uri", uri.toString());
            call.resolve(result);
        } catch (Exception error) {
            if (uri != null) getContext().getContentResolver().delete(uri, null, null);
            call.reject("保存图片失败：" + error.getMessage());
        }
    }

    @ActivityCallback
    private void saveDocument(PluginCall call, ActivityResult result) {
        if (call == null) return;
        Intent intent = result.getData();
        Uri uri = intent == null ? null : intent.getData();
        if (result.getResultCode() != Activity.RESULT_OK || uri == null) {
            JSObject canceled = new JSObject();
            canceled.put("canceled", true);
            call.resolve(canceled);
            return;
        }
        try (OutputStream output = getContext().getContentResolver().openOutputStream(uri)) {
            if (output == null) throw new Exception("无法打开图片文件");
            output.write(Base64.decode(call.getString("data", ""), Base64.DEFAULT));
            JSObject saved = new JSObject();
            saved.put("saved", true);
            saved.put("uri", uri.toString());
            call.resolve(saved);
        } catch (Exception error) { call.reject("保存图片失败：" + error.getMessage()); }
    }
}
