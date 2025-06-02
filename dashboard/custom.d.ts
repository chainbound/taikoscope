/// <reference types="vite/client" />

declare module '*.css';

interface ImportMetaEnv {
  readonly VITE_API_BASE?: string;
  readonly API_BASE?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
