import { flushSync } from 'react-dom'
import { createRoot } from 'react-dom/client'

import { InputControl, type InputControlProps } from './input-control'

/**
 * 输入框岛 —— 两种用法：
 *
 * 1. **自动增强**（推荐，绝大多数场景）：在原生 `<input>` 上加 `data-island-input`，
 *    岛在加载时把它原地升级成组件库的标准 Input。原元素的 id 会转给渲染出的 input，
 *    所以业务侧 `$('id').value` / `el.addEventListener('input', …)` 一行都不用改。
 *    再加 `data-island-icon="⌕"` 就升级成 InputGroup（图标走标准的 InputGroupAddon），
 *    业务侧原本为图标准备的 `.input-affix` 外壳会被就地转正、无需改动调用方。
 *    弹窗、表格、代理表单里动态生成的输入框由 MutationObserver 接住，页面无需
 *    任何手动调用 —— 与 select.js / desktop-tooltip.tsx 的「自动增强」是同一个约定。
 *
 * 2. **命令式挂载**：`mount(host, options)` —— 宿主元素整个交给岛渲染，初值由
 *    业务侧以参数带进去（如 models 页的搜索框，它的宿主是 `<div class="island">`）。
 *
 * 两条路都是**非受控**的，值由浏览器持有；外部要改值走返回的 `setValue`。
 * 理由见 input-control.tsx 顶部（受控会让打字等 React 的 flush，重绘慢时发涩）。
 */

type InputIslandOptions = {
  /** 初始值 */
  value?: string
  placeholder?: string
  /** 原生 input 类型，默认 search */
  type?: string
  /** 无障碍名。搜索框只有 placeholder 时读屏播报偏弱，建议给一个 */
  ariaLabel?: string
  /** 前缀图标字符（如 ⌕） */
  icon?: string
  onInput?: (value: string) => void
}

type InputIsland = {
  /** 命令式设值（清空 / 回填）。非受控组件只能这样改值，改完不触发 onInput。 */
  setValue(value: string): void
  /** 当前值（外部懒得自己记时用；正常场景业务侧本就有那份状态） */
  getValue(): string
  focus(): void
  unmount(): void
}

/** 把宿主元素整个交给岛渲染 */
function mountInput(host: HTMLElement, options: InputIslandOptions): InputIsland {
  const root = createRoot(host)
  /** React 提交后才填得上；mount 返回时若立刻调 setValue 可能还没就绪 */
  const domRef: { current: HTMLInputElement | null } = { current: null }

  root.render(
    <InputControl
      domRef={domRef}
      defaultValue={options.value}
      placeholder={options.placeholder}
      // 默认 search：挂载点是空 div，没有「原元素」可继承类型。这个默认值不能省 ——
      // 无 type 的 <input> 不匹配任何 `input[type=…]` 属性选择器（属性选择器要求
      // 属性存在），项目的整套表单样式与 flex: 1 1 auto 会一起落空，输入框掉回
      // 浏览器默认外观、宽度塌成内容宽。
      type={options.type ?? 'search'}
      icon={options.icon}
      aria-label={options.ariaLabel}
      onInput={options.onInput}
    />
  )

  return {
    setValue(value: string) {
      const element = domRef.current
      if (element && element.value !== value) element.value = value
    },
    getValue() {
      return domRef.current?.value ?? ''
    },
    focus() {
      domRef.current?.focus()
    },
    unmount() {
      root.unmount()
    },
  }
}

/** 原 `<input>` 上「配置性」属性到岛 props 的搬运表（状态与事件由岛自己管，不搬） */
const CARRIED_ATTRS = [
  ['id', 'id'],
  ['type', 'type'],
  ['name', 'name'],
  ['title', 'title'],
  ['placeholder', 'placeholder'],
  ['min', 'min'],
  ['max', 'max'],
  ['step', 'step'],
  ['autocomplete', 'autoComplete'],
] as const

function readInputProps(el: HTMLInputElement): Omit<InputControlProps, 'domRef'> {
  const props: Omit<InputControlProps, 'domRef'> = {
    // 非受控：原元素的现值就是岛的初值
    defaultValue: el.value || undefined,
    inputClassName: el.className || undefined,
  }
  for (const [attr, key] of CARRIED_ATTRS) {
    const value = el.getAttribute(attr)
    if (value != null) (props as Record<string, unknown>)[key] = value
  }
  const maxLength = el.getAttribute('maxlength')
  if (maxLength != null) props.maxLength = Number(maxLength)
  const inputMode = el.getAttribute('inputmode')
  if (inputMode != null) props.inputMode = inputMode as InputControlProps['inputMode']
  if (el.hasAttribute('spellcheck')) props.spellCheck = el.spellcheck
  if (el.disabled) props.disabled = true
  if (el.readOnly) props.readOnly = true
  // 初值要带上：如模型映射里的「自定义等级」输入框出厂是 hidden 的，由
  // models-reasoning.js 按下拉选中项切换（直接写 .hidden / .disabled，操作的是
  // DOM 属性，升级后照样生效）。漏搬它，这个藏起来的框一升级就会露出来。
  if (el.hidden) props.hidden = true
  const ariaLabel = el.getAttribute('aria-label')
  if (ariaLabel != null) props['aria-label'] = ariaLabel
  // 业务侧的 data-* 钩子原样保留：如任务面板靠 `[data-task-interval]` 选元素、
  // 事件委托靠 `target.matches('[data-task-interval]')` 认元素，属性一丢这些全失效。
  // data-island-* 是岛自己的指令，不外传。
  for (const attr of el.getAttributeNames()) {
    if (!attr.startsWith('data-') || attr.startsWith('data-island-')) continue
    ;(props as Record<string, unknown>)[attr] = el.getAttribute(attr) ?? ''
  }
  const icon = el.getAttribute('data-island-icon')
  if (icon != null) props.icon = icon
  return props
}

/**
 * 原地升级一个 `<input data-island-input>`。
 *
 * 用 `flushSync` 同步提交：岛是在脚本加载期批量升级的，而业务模块紧随其后加载
 * 并绑定事件 —— 若渲染异步落到微任务里，业务侧 `$('id')` 会先拿到 null。
 * 同步提交后，升级完成的那一刻新 input 就带着原 id 在位了。
 *
 * 宿主怎么选：老结构是「业务壳 `.input-affix` + 里面的原生 input」，而带图标的
 * 输入框现在渲染成标准的 InputGroup —— 它自带容器与边框，外壳再套一层会让输入框
 * 的内边距双重让位（`.input-affix input` 的 28px 叠上 addon 的占位）。所以外壳
 * **就地转正**：去掉 `input-affix` 类、补回它的 flex 布局，业务类与 id 原样留在
 * 外壳上（`.fm-search { width: 240px }` 这类宽度规则、`#add-search-wrap` 这类
 * 显隐开关都挂在它们身上，一转移就失效）。没有外壳时另建一个 `display: contents`
 * 的宿主 —— 它不生成盒子，岛渲染出的控件直接参与父级布局，接入前后布局等价。
 */
function upgradeInput(el: HTMLInputElement) {
  if (el.dataset.islandUpgraded) return
  // 标记留在原元素上：即使它已被换下，观察器再遇到也不会重复处理
  el.dataset.islandUpgraded = '1'

  const props = readInputProps(el)
  const shell = el.parentElement?.classList.contains('input-affix') ? el.parentElement : null

  let host: HTMLElement
  if (shell) {
    shell.className = shell.className.replace(/\binput-affix\b/, '').trim()
    shell.style.display = 'flex'
    shell.style.alignItems = 'center'
    host = shell
  } else {
    host = document.createElement('span')
    host.style.display = 'contents'
    el.replaceWith(host)
  }

  const domRef: { current: HTMLInputElement | null } = { current: null }
  const root = createRoot(host)
  flushSync(() => {
    // React 首次渲染会清空容器里的旧内容（原 input），不必手动摘
    root.render(<InputControl {...props} domRef={domRef} />)
  })
  // 岛不接管值，只渲染控件；root 随宿主元素一起被移除，无需手动 unmount
  void root
}

/** 扫描 root 下所有待升级的输入框（含 root 自身） */
function autoMountInputs(root: ParentNode = document) {
  if (root instanceof HTMLInputElement && root.matches('[data-island-input]')) {
    upgradeInput(root)
    return
  }
  root.querySelectorAll<HTMLInputElement>('input[data-island-input]').forEach(upgradeInput)
}

/**
 * 开始自动增强：先处理页面里已有的，再盯着后续动态生成的
 * （弹窗、表格行、代理表单都是运行时拼出来的 HTML）。
 *
 * 观察器不会自激：升级产出的是 `<span>`，不匹配 `input[data-island-input]`。
 */
function observeInputs() {
  autoMountInputs()
  const observer = new MutationObserver(records => {
    for (const record of records) {
      for (const node of record.addedNodes) {
        if (node instanceof Element) autoMountInputs(node)
      }
    }
  })
  observer.observe(document.body, { childList: true, subtree: true })
  return observer
}

export {
  autoMountInputs,
  mountInput,
  observeInputs,
  type InputIsland,
  type InputIslandOptions,
}
