/**
 * 核心类型定义
 */

// Tauri 命令返回类型
export interface CommandResult {
  output: string;
}

export interface Process {
  pid: number;
  name: string;
  cpu: number;
  memory: number;
  status: string;
}

export interface FileInfo {
  name: string;
  path: string;
  file_type: 'directory' | 'file';
  size: string;
  modified: string;
}

export interface LogEntry {
  timestamp: string;
  level: string;
  source: string;
  message: string;
}

export interface CommandHistory {
  command: string;
  timestamp: string;
  status: string;
  id: string;
}

// 应用设置
export interface AppSettings {
  defaultProfile: string;
  launchSize: string;
  autoScroll: boolean;
  copyFormatting: boolean;
  detectUrls: boolean;
  colorScheme: string;
  fontFamily: string;
  fontSize: number;
  opacity: number;
  predictiveInput: boolean;
  historyEnabled: boolean;
  historySize: number;
  bellEnabled: boolean;
}

// 功能模块接口
export interface FeatureModule {
  name: string;
  initialize(): Promise<void>;
  destroy?(): void | Promise<void>;
}

export interface PluginCapabilities {
  usesGlobalActions?: boolean;
  usesTauri?: boolean;
  windowActionNamespace?: string;
}

export interface PluginContext {
  pluginId: string;
}

// 插件接口
export interface Plugin {
  id: string;
  name: string;
  version: string;
  description?: string;
  capabilities?: PluginCapabilities;
  initialize(context?: PluginContext): Promise<void>;
  destroy?(): void | Promise<void>;
}

// 插件元数据
export interface PluginMetadata {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  entry: string; // 入口文件路径
  capabilities?: PluginCapabilities;
}

export interface PluginDiagnostic extends PluginMetadata {
  loaded: boolean;
  usesNamespace: boolean;
}
