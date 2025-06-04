// Navigation utility functions for robust URL handling

export const isValidUrl = (urlString: string): boolean => {
  try {
    if (!urlString || urlString.trim() === '') {
      return false;
    }
    
    // Check if it's a relative URL by trying to parse it with base
    new URL(urlString, window.location.origin);
    
    // If it starts with a protocol but isn't http/https, reject it
    if (urlString.includes('://') && !urlString.startsWith('http://') && !urlString.startsWith('https://')) {
      return false;
    }
    
    // Reject obviously invalid patterns
    if (urlString.includes('javascript:') || urlString.includes('data:') || urlString.includes('vbscript:')) {
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
    
    return urlObj.toString();
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
    
    return true;
  } catch {
    return false;
  }
};

export const cleanSearchParams = (params: URLSearchParams): URLSearchParams => {
  const cleaned = new URLSearchParams();
  
  try {
    // Only keep known valid parameters
    const allowedParams = ['view', 'table', 'sequencer', 'address', 'page', 'start', 'end', 'range'];
    
    for (const [key, value] of params.entries()) {
      if (allowedParams.includes(key) && value.trim()) {
        cleaned.set(key, value.trim());
      }
    }
    
    return cleaned;
  } catch {
    return new URLSearchParams();
  }
};