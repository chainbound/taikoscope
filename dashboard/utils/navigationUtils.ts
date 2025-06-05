// Navigation utility functions for robust URL handling

export const isValidUrl = (urlString: string): boolean => {
  try {
    if (!urlString || urlString.trim() === '' || /\s/.test(urlString)) {
      return false;
    }

    // Basic traversal checks
    if (urlString.includes('..')) {
      return false;
    }

    // Check if it's a relative URL by trying to parse it with base
    new URL(urlString, window.location.origin);

    // If it starts with a protocol but isn't http/https, reject it
    if (
      urlString.includes('://') &&
      !urlString.startsWith('http://') &&
      !urlString.startsWith('https://')
    ) {
      return false;
    }

    // Reject obviously invalid patterns
    if (/^(javascript|data|vbscript):/i.test(urlString)) {
      return false;
    }

    return true;
  } catch {
    return false;
  }
};

export const sanitizeUrl = (url: string | URL): string => {
  try {
    let urlObj: URL;

    if (url instanceof URL) {
      urlObj = url;
    } else {
      // First check if it's a valid URL string
      if (!isValidUrl(url)) {
        console.error('Invalid URL provided for sanitization:', url);
        return window.location.pathname;
      }

      // Handle relative URLs by using current origin as base
      urlObj = new URL(url, window.location.origin);
    }

    // Ensure we're staying within the same origin
    if (urlObj.origin !== window.location.origin) {
      console.warn('Attempted navigation to different origin:', urlObj.origin);
      return window.location.pathname;
    }

    // Sanitize search parameters and drop fragments
    urlObj.search = cleanSearchParams(urlObj.searchParams).toString();
    urlObj.hash = '';

    // Return relative path for SPA navigation
    return urlObj.pathname + urlObj.search;
  } catch (err) {
    console.error('Failed to sanitize URL:', err);
    return window.location.pathname;
  }
};

export const createSafeUrl = (baseUrl?: string): URL => {
  try {
    return new URL(baseUrl || window.location.href);
  } catch {
    // Fallback to a clean URL if the current one is corrupted
    return new URL(window.location.pathname, window.location.origin);
  }
};

export const validateSearchParams = (params: URLSearchParams): boolean => {
  try {
    // Check for reasonable parameter values
    const view = params.get('view');
    if (view && !['table', 'economics'].includes(view)) {
      console.warn('Invalid view parameter:', view);
      return false;
    }

    const page = params.get('page');
    if (page && (isNaN(Number(page)) || Number(page) < 0)) {
      console.warn('Invalid page parameter:', page);
      return false;
    }

    const range = params.get('range');
    if (range && !['1h', '24h', '7d'].includes(range)) {
      console.warn('Invalid range parameter:', range);
      return false;
    }

    const table = params.get('table');
    if (table && !/^[a-zA-Z0-9_-]+$/.test(table)) {
      console.warn('Invalid table parameter:', table);
      return false;
    }

    const sort = params.get('sort');
    if (sort && !['asc', 'desc'].includes(sort)) {
      console.warn('Invalid sort parameter:', sort);
      return false;
    }

    const filter = params.get('filter');
    if (filter && !/^[a-zA-Z0-9_-]+$/.test(filter)) {
      console.warn('Invalid filter parameter:', filter);
      return false;
    }

    return true;
  } catch {
    return false;
  }
};

export const cleanSearchParams = (params: URLSearchParams): URLSearchParams => {
  const cleaned = new URLSearchParams();

  try {
    const validators: Record<string, (v: string) => boolean> = {
      view: (v) => ['table', 'economics'].includes(v),
      page: (v) => /^\d+$/.test(v),
      start: (v) => /^\d+$/.test(v),
      end: (v) => /^\d+$/.test(v),
      range: (v) => ['1h', '24h', '7d'].includes(v),
      sequencer: (v) => /^[0-9a-zA-Z]+$/.test(v),
      address: (v) => /^[0-9a-zA-Z]+$/.test(v),
      table: (v) => /^[a-zA-Z0-9_-]+$/.test(v),
      sort: (v) => ['asc', 'desc'].includes(v),
      filter: (v) => /^[a-zA-Z0-9_-]+$/.test(v),
    };

    for (const [key, value] of params.entries()) {
      const trimmed = value.trim();
      const validate = validators[key];
      if (validate && trimmed && validate(trimmed)) {
        cleaned.set(key, trimmed);
      }
    }

    return cleaned;
  } catch {
    return new URLSearchParams();
  }
};

export const safeNavigate = (
  routerNavigate: (path: string, opts?: { replace?: boolean }) => void,
  url: string | URL,
  replace = false,
) => {
  const sanitized = sanitizeUrl(url);
  routerNavigate(sanitized, { replace });
};
