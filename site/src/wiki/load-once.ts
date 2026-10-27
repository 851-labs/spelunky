// A chunk the browser loads the first time it's asked for (the search index, the hover cards, the popups'
// code): loaded once, and tried again after a failure (the network, say).
export function loadOnce<T>(load: () => Promise<T>): () => Promise<T> {
  let loading: Promise<T> | null = null;
  return () =>
    (loading ??= load().catch((error) => {
      loading = null;
      throw error;
    }));
}

// What the Worker gets instead of a chunk it never shows: `import.meta.env.SSR ? never() : import(...)`,
// which the bundler folds, so the Worker's bundle hasn't got the chunk.
export const never = () => new Promise<never>(() => {});
