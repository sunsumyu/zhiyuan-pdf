/**
 * 插件加载器
 * 负责动态加载和管理插件
 */

import type { Plugin, PluginContext, PluginMetadata } from './types';
import { eventBus } from './event-bus';
import { getPluginCatalogEntry } from './plugin-catalog';
import type { PluginDiagnostic } from './types';

class PluginLoader {
  private plugins: Map<string, Plugin> = new Map();
  private loadedPlugins: Set<string> = new Set();
  private readonly reservedWindowActionNamespaces = new Set(['app', 'core', 'system']);

  private createContext(plugin: Plugin): PluginContext {
    return { pluginId: plugin.id };
  }

  private validatePlugin(plugin: Plugin): void {
    if (!plugin.id.trim()) {
      throw new Error('Plugin id is required');
    }

    if (!/^[a-z0-9-]+$/.test(plugin.id)) {
      throw new Error(`Plugin id "${plugin.id}" must use lowercase letters, numbers, or dashes`);
    }

    const namespace = plugin.capabilities?.windowActionNamespace;
    if (!namespace) return;

    if (!/^[a-z0-9-]+$/.test(namespace)) {
      throw new Error(`Plugin windowActionNamespace "${namespace}" must use lowercase letters, numbers, or dashes`);
    }

    if (this.reservedWindowActionNamespaces.has(namespace)) {
      throw new Error(`Plugin windowActionNamespace "${namespace}" is reserved`);
    }

    const namespaceTaken = Array.from(this.plugins.values()).some(
      (existingPlugin) => existingPlugin.capabilities?.windowActionNamespace === namespace,
    );
    if (namespaceTaken) {
      throw new Error(`Plugin windowActionNamespace "${namespace}" is already registered`);
    }
  }

  /**
   * 注册插件
   */
  register(plugin: Plugin): void {
    if (this.plugins.has(plugin.id)) {
      console.warn(`Plugin ${plugin.id} already registered`);
      return;
    }

    this.validatePlugin(plugin);
    this.plugins.set(plugin.id, plugin);
    console.log(`[PluginLoader] Registered plugin: ${plugin.name} v${plugin.version}`);
  }

  /**
   * 加载插件
   */
  async load(pluginId: string): Promise<void> {
    const plugin = this.plugins.get(pluginId);

    if (!plugin) {
      throw new Error(`Plugin ${pluginId} not found`);
    }

    if (this.loadedPlugins.has(pluginId)) {
      console.warn(`Plugin ${pluginId} already loaded`);
      return;
    }

    try {
      await plugin.initialize(this.createContext(plugin));
      this.loadedPlugins.add(pluginId);
      eventBus.emit('plugin:loaded', { pluginId, plugin });
      console.log(`[PluginLoader] Loaded plugin: ${plugin.name}`);
    } catch (error) {
      console.error(`[PluginLoader] Failed to load plugin ${pluginId}:`, error);
      throw error;
    }
  }

  /**
   * 卸载插件
   */
  async unload(pluginId: string): Promise<void> {
    const plugin = this.plugins.get(pluginId);

    if (!plugin) {
      throw new Error(`Plugin ${pluginId} not found`);
    }

    if (!this.loadedPlugins.has(pluginId)) {
      console.warn(`Plugin ${pluginId} not loaded`);
      return;
    }

    try {
      if (plugin.destroy) {
        await plugin.destroy();
      }
      this.loadedPlugins.delete(pluginId);
      eventBus.emit('plugin:unloaded', { pluginId, plugin });
      console.log(`[PluginLoader] Unloaded plugin: ${plugin.name}`);
    } catch (error) {
      console.error(`[PluginLoader] Failed to unload plugin ${pluginId}:`, error);
      throw error;
    }
  }

  /**
   * 获取所有已注册的插件
   */
  getAll(): Plugin[] {
    return Array.from(this.plugins.values());
  }

  /**
   * 获取已加载的插件
   */
  getLoaded(): Plugin[] {
    return Array.from(this.loadedPlugins)
      .map(id => this.plugins.get(id))
      .filter((p): p is Plugin => p !== undefined);
  }

  getMetadata(): PluginMetadata[] {
    return this.getAll().map((plugin) => ({
      author: getPluginCatalogEntry(plugin.id)?.author || 'unknown',
      capabilities: plugin.capabilities,
      description: plugin.description || '',
      entry: getPluginCatalogEntry(plugin.id)?.entry || '',
      id: plugin.id,
      name: plugin.name,
      version: plugin.version,
    }));
  }

  getDiagnostics(): PluginDiagnostic[] {
    return this.getMetadata().map((metadata) => ({
      ...metadata,
      loaded: this.loadedPlugins.has(metadata.id),
      usesNamespace: !!metadata.capabilities?.windowActionNamespace,
    }));
  }

  /**
   * 检查插件是否已加载
   */
  isLoaded(pluginId: string): boolean {
    return this.loadedPlugins.has(pluginId);
  }
}

// 导出单例
export const pluginLoader = new PluginLoader();
