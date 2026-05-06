/**
 * Core Interfaces for Dependency Injection
 */

export interface ITemplateLoader {
    loadComponent(path: string): Promise<string>;
    injectComponent(containerId: string, componentPath: string): Promise<void>;
    injectContent(containerId: string, content: string): void;
    replaceComponent(placeholderId: string, componentPath: string): Promise<void>;
    replaceComponentContent(placeholderId: string, html: string): Promise<void>;
}

export interface IVisualizer {
    init(): Promise<void>;
    pause?(): void;
    resume?(): void;
    destroy?(): void;
}

export interface IAlgorithmManager {
    initialize(): void;
    registerAlgorithm(config: any): void;
    selectAlgorithm(id: string): Promise<void>;
}
