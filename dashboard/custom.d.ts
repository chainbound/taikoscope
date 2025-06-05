/// <reference types="vite/client" />

declare module '*.css';

interface ImportMetaEnv {
  readonly VITE_API_BASE?: string;
  readonly API_BASE?: string;
  readonly VITE_NETWORK_NAME?: string;
  readonly NETWORK_NAME?: string;
  readonly VITE_TAIKOSCAN_BASE?: string;
  readonly TAIKOSCAN_BASE?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
