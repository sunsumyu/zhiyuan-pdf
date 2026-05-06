import type { PluginMetadata } from './types';

const builtinPluginCatalog: Record<string, Omit<PluginMetadata, 'capabilities' | 'description' | 'id' | 'name' | 'version'>> = {
  'ai-chat': {
    author: 'unknown',
    entry: 'src/plugins/ai-chat/index.ts',
  },
  'algorithm-viz': {
    author: 'unknown',
    entry: 'src/plugins/algorithm-viz/index.ts',
  },
  dictionary: {
    author: 'unknown',
    entry: 'src/plugins/dictionary/index.ts',
  },
  editor: {
    author: 'unknown',
    entry: 'src/plugins/editor/index.ts',
  },
  'english-master': {
    author: 'unknown',
    entry: 'src/plugins/english-master/index.ts',
  },
  game: {
    author: 'unknown',
    entry: 'src/plugins/minesweeper/index.ts',
  },
  jvm: {
    author: 'unknown',
    entry: 'src/plugins/jvm/index.ts',
  },
  loadtest: {
    author: 'unknown',
    entry: 'src/plugins/loadtest/index.ts',
  },
  'neuron-sim': {
    author: 'unknown',
    entry: 'src/plugins/neuron-sim/index.ts',
  },
  'super-brain-demo': {
    author: 'unknown',
    entry: 'src/plugins/super-brain-demo/index.ts',
  },
  'pdf-viewer': {
    author: 'unknown',
    entry: 'src/plugins/pdf-viewer/index.ts',
  },
  'neuron-memory-engine': {
    author: 'Antigravity',
    entry: 'src/plugins/neuron-memory-engine/index.ts',
  },
  'video-player': {
    author: 'unknown',
    entry: 'src/plugins/video-player/index.ts',
  },
};

export function getPluginCatalogEntry(pluginId: string) {
  return builtinPluginCatalog[pluginId];
}
