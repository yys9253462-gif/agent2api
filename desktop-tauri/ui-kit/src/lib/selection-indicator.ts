import * as React from 'react'

const INITIAL = { left: 0, top: 0, width: 0, height: 0, ready: false }

type SelectionRect = typeof INITIAL

function useSelectionIndicator(itemSelector: string, activeSelector: string) {
  const containerRef = React.useRef<HTMLDivElement>(null)
  const [rect, setRect] = React.useState<SelectionRect>(INITIAL)
  const [animate, setAnimate] = React.useState(false)
  const positioned = React.useRef(false)
  const frame = React.useRef<number | null>(null)

  const measure = React.useCallback(() => {
    const container = containerRef.current
    const item = container?.querySelector<HTMLElement>(activeSelector)
    if (!container || !item || !container.offsetWidth || !container.offsetHeight || !item.getClientRects().length) {
      positioned.current = false
      setAnimate(false)
      setRect(current => current.ready ? { ...current, ready: false } : current)
      return
    }
    const outer = container.getBoundingClientRect()
    const inner = item.getBoundingClientRect()
    if (!outer.width || !outer.height || !inner.width || !inner.height) return
    const scaleX = outer.width / container.offsetWidth
    const scaleY = outer.height / container.offsetHeight
    const next = {
      left: (inner.left - outer.left) / scaleX - container.clientLeft + container.scrollLeft,
      top: (inner.top - outer.top) / scaleY - container.clientTop + container.scrollTop,
      width: inner.width / scaleX,
      height: inner.height / scaleY,
      ready: true,
    }
    positioned.current = true
    setRect(current => current.ready
      && Math.abs(current.left - next.left) < 0.1 && Math.abs(current.top - next.top) < 0.1
      && Math.abs(current.width - next.width) < 0.1 && Math.abs(current.height - next.height) < 0.1
      ? current : next)
  }, [activeSelector])

  const scheduleMeasure = React.useCallback(() => {
    if (!positioned.current) {
      measure()
      return
    }
    if (frame.current !== null) return
    frame.current = requestAnimationFrame(() => {
      frame.current = null
      measure()
    })
  }, [measure])

  React.useLayoutEffect(() => { scheduleMeasure() })

  React.useLayoutEffect(() => {
    const container = containerRef.current
    if (!container) return
    const resize = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(scheduleMeasure)
    const observeItems = () => {
      resize?.disconnect()
      resize?.observe(container)
      container.querySelectorAll(itemSelector).forEach(item => resize?.observe(item))
    }
    observeItems()
    const mutation = new MutationObserver(records => {
      const changes = records.filter(record => !(record.target instanceof Element && record.target.hasAttribute('data-selection-indicator')))
      if (!changes.length) return
      if (changes.some(record => record.type === 'childList')) observeItems()
      scheduleMeasure()
    })
    mutation.observe(container, {
      subtree: true, childList: true, characterData: true, attributes: true,
      attributeFilter: ['data-active', 'data-checked', 'class', 'hidden', 'style'],
    })
    window.addEventListener('resize', scheduleMeasure)
    document.fonts?.addEventListener('loadingdone', scheduleMeasure)
    return () => {
      resize?.disconnect()
      mutation.disconnect()
      if (frame.current !== null) cancelAnimationFrame(frame.current)
      frame.current = null
      window.removeEventListener('resize', scheduleMeasure)
      document.fonts?.removeEventListener('loadingdone', scheduleMeasure)
    }
  }, [itemSelector, scheduleMeasure])

  React.useEffect(() => {
    if (!rect.ready || animate) return
    let second = 0
    const first = requestAnimationFrame(() => {
      second = requestAnimationFrame(() => {
        if (positioned.current) setAnimate(true)
      })
    })
    return () => { cancelAnimationFrame(first); cancelAnimationFrame(second) }
  }, [rect.ready, animate])

  return {
    containerRef,
    indicator: {
      'data-selection-indicator': '',
      'data-ready': rect.ready ? 'true' : 'false',
      'data-animate': animate ? 'true' : 'false',
      style: {
        transform: `translate3d(${rect.left}px, ${rect.top}px, 0)`,
        width: rect.width,
        height: rect.height,
      },
    },
  }
}

export { useSelectionIndicator }
