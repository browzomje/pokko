export interface Manga {
  title: string;
  url: string;
  cover: string;
  source_id: string;
  source_name: string;
  language: string;
  authors?: string[];
}
export interface Chapter {
  page_count?: number | null;
  title: string;
  url: string;
}
export interface Volume {
  title: string;
  chapters: Chapter[];
}
export interface Detail {
  manga: Manga;
  description: string;
  volumes: Volume[];
}
export interface Settings {
  base_url: string;
  output: string;
  delay_ms: number;
  cbz: boolean;
  export_format: string;
  sources: Source[];
}
export interface Job {
  verification?: {
    state: string;
    verified: number;
    total: number;
    repaired: number;
  };
  id: string;
  archived?: boolean;
  manga: Manga;
  volume: Volume;
  status: string;
  chapter: string;
  chapters_done: number;
  pages_done: number;
  pages_total: number;
  bytes: number;
  message: string;
  output: string;
  settings: Settings;
}

export interface Source {
  category: string;
  id: string;
  name: string;
  base_url: string;
  language: string;
  adapter: string;
  enabled: boolean;
  min_delay_ms: number;
  note: string;
  search_path: string;
  result_selector: string;
  chapter_selector: string;
  image_selector: string;
  description_selector: string;
  next_selector: string;
  reader_next_selector?: string;
  author_selector?: string;
  workflow?: { kind: string; url: string; selector: string }[];
}
export interface SourceReport {
  id: string;
  name: string;
  count: number;
  has_next: boolean;
  error: string | null;
}

export interface MangaPage {
  id: string;
  title: string;
  manga: Manga;
  detail: Detail | null;
  loading: boolean;
  error: string;
  selected: string[];
  expanded: string[];
  catalog: string;
  volumeFilter: string;
}
