export interface FileNode {
  name: string;
  path: string;
  size: number;
  modified_at: number; // Unix timestamp seconds
  is_dir: boolean;
  is_symlink: boolean;
  symlink_target?: string;
  permissions: string;
}

export type TransferDirection = "upload" | "download";

export interface SearchHit {
  path: string;
  rel: string;
  name: string;
  is_dir: boolean;
}
