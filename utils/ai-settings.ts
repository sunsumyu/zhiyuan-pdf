/**
 * AI settings persistence via Tauri FS plugin.
 * Stores/loads Gemini API key and other AI-related preferences.
 */
import { readTextFile, writeTextFile, BaseDirectory } from '@tauri-apps/plugin-fs';

export interface AiSettings {
  geminiApiKey?: string;
}

const AI_SETTINGS_FILE = 'ai-settings.json';

export async function loadAiSettings(): Promise<AiSettings> {
  try {
    const text = await readTextFile(AI_SETTINGS_FILE, { baseDir: BaseDirectory.AppConfig });
    return JSON.parse(text) as AiSettings;
  } catch {
    return {};
  }
}

export async function saveAiSettings(settings: AiSettings): Promise<void> {
  const existing = await loadAiSettings();
  const merged = { ...existing, ...settings };
  await writeTextFile(AI_SETTINGS_FILE, JSON.stringify(merged, null, 2), {
    baseDir: BaseDirectory.AppConfig,
  });
}
