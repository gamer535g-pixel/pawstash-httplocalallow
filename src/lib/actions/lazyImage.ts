export interface LazyImageOptions {
  src: string;
  root?: HTMLElement | null;
}

// Archive and cloud previews come from rate-limited hosts, so loads wait for visibility and are capped.
export function createLazyImageLoader(concurrency: number) {
  let active = 0;
  const waiting: Array<() => void> = [];

  function pump() {
    while (active < concurrency && waiting.length > 0) {
      waiting.shift()!();
    }
  }

  return function lazyImage(node: HTMLImageElement, options: LazyImageOptions) {
    let current = options;
    let observer: IntersectionObserver | null = null;
    let pending: (() => void) | null = null;
    let inFlight = false;
    let loadedSrc = '';

    function release() {
      if (!inFlight) return;
      inFlight = false;
      active--;
      pump();
    }

    function start() {
      pending = null;
      if (!current.src) return;
      inFlight = true;
      active++;
      loadedSrc = current.src;
      node.src = current.src;
    }

    function dequeue() {
      if (!pending) return;
      const index = waiting.indexOf(pending);
      if (index >= 0) waiting.splice(index, 1);
      pending = null;
    }

    function enqueue() {
      if (pending || inFlight || !current.src || loadedSrc === current.src) return;
      pending = start;
      waiting.push(start);
      pump();
    }

    function observe() {
      observer?.disconnect();
      observer = new IntersectionObserver(
        (entries) => {
          for (const entry of entries) {
            if (entry.isIntersecting) enqueue();
            else dequeue();
          }
        },
        { root: current.root ?? null, rootMargin: '50% 0px' }
      );
      observer.observe(node);
    }

    node.addEventListener('load', release);
    node.addEventListener('error', release);
    observe();

    return {
      update(next: LazyImageOptions) {
        const srcChanged = next.src !== current.src;
        const rootChanged = next.root !== current.root;
        current = next;
        if (srcChanged) {
          dequeue();
          release();
          loadedSrc = '';
          node.removeAttribute('src');
        }
        if (srcChanged || rootChanged) observe();
      },
      destroy() {
        observer?.disconnect();
        dequeue();
        release();
        node.removeEventListener('load', release);
        node.removeEventListener('error', release);
      }
    };
  };
}
