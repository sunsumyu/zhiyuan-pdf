/**
 * Template Loader
 * Responsible for loading and caching HTML components
 */

import { ITemplateLoader } from './interfaces';

export class TemplateLoader implements ITemplateLoader {
    private static instance: TemplateLoader;
    private cache: Map<string, string> = new Map();
    private loading: Map<string, Promise<string>> = new Map();

    public constructor() { }

    /**
     * @deprecated Use dependency injection instead.
     */
    public static getInstance(): TemplateLoader {
        if (!TemplateLoader.instance) {
            TemplateLoader.instance = new TemplateLoader();
        }
        return TemplateLoader.instance;
    }

    /**
     * Load a component by path
     * @param path Path to the HTML file (relative to src/)
     */
    public async loadComponent(path: string): Promise<string> {
        if (this.cache.has(path)) {
            return this.cache.get(path)!;
        }

        if (this.loading.has(path)) {
            return this.loading.get(path)!;
        }

        const loadPromise = fetch(path)
            .then(response => {
                if (!response.ok) {
                    throw new Error(`Failed to load template: ${path}`);
                }
                return response.text();
            })
            .then(html => {
                this.cache.set(path, html);
                this.loading.delete(path);
                return html;
            })
            .catch(err => {
                this.loading.delete(path);
                throw err;
            });

        this.loading.set(path, loadPromise);
        return loadPromise;
    }

    /**
     * Inject a component into a container
     * @param containerId ID of the container element
     * @param componentPath Path to the component HTML
     */
    public async injectComponent(containerId: string, componentPath: string): Promise<void> {
        const container = document.getElementById(containerId);
        if (!container) {
            console.error(`Container not found: ${containerId}`);
            return;
        }

        try {
            const html = await this.loadComponent(componentPath);
            container.innerHTML = html;
            this.executeScripts(container);
        } catch (error) {
            console.error(`Error injecting component ${componentPath}:`, error);
            container.innerHTML = `<div class="error">Failed to load component</div>`;
        }
    }

    /**
     * Inject raw content into a container
     * @param containerId ID of the container element
     * @param content Raw HTML content
     */
    public injectContent(containerId: string, content: string): void {
        const container = document.getElementById(containerId);
        if (!container) {
            console.error(`Container not found: ${containerId}`);
            return;
        }

        try {
            container.innerHTML = content;
            this.executeScripts(container);
        } catch (error) {
            console.error(`Error injecting content into ${containerId}:`, error);
            container.innerHTML = `<div class="error">Failed to inject content</div>`;
        }
    }

    /**
     * Replace a placeholder with a component
     * @param placeholderId ID of the placeholder element
     * @param componentPath Path to the component HTML
     */
    public async replaceComponent(placeholderId: string, componentPath: string): Promise<void> {
        const placeholder = document.getElementById(placeholderId);
        if (!placeholder) {
            console.error(`Placeholder not found: ${placeholderId}`);
            return;
        }

        try {
            const html = await this.loadComponent(componentPath);
            const temp = document.createElement('div');
            temp.innerHTML = html;
            const newElement = temp.firstElementChild;

            if (newElement) {
                placeholder.replaceWith(newElement);
                this.executeScripts(newElement as HTMLElement);
            } else {
                console.error(`Component ${componentPath} is empty or invalid`);
            }
        } catch (error) {
            console.error(`Error replacing component ${componentPath}:`, error);
        }
    }

    /**
     * Replace a placeholder with raw component content
     * @param placeholderId ID of the placeholder element
     * @param html Raw HTML content of the component
     */
    public async replaceComponentContent(placeholderId: string, html: string): Promise<void> {
        const placeholder = document.getElementById(placeholderId);
        if (!placeholder) {
            console.error(`Placeholder not found: ${placeholderId}`);
            return;
        }

        try {
            const temp = document.createElement('div');
            temp.innerHTML = html;
            const newElement = temp.firstElementChild;

            if (newElement) {
                placeholder.replaceWith(newElement);
                this.executeScripts(newElement as HTMLElement);
            } else {
                console.error(`Component content for ${placeholderId} is empty or invalid`);
            }
        } catch (error) {
            console.error(`Error replacing component content for ${placeholderId}:`, error);
        }
    }

    private executeScripts(container: HTMLElement): void {
        const scripts = container.querySelectorAll('script');
        scripts.forEach(oldScript => {
            const newScript = document.createElement('script');
            Array.from(oldScript.attributes).forEach(attr => {
                newScript.setAttribute(attr.name, attr.value);
            });
            newScript.appendChild(document.createTextNode(oldScript.innerHTML));
            oldScript.parentNode?.replaceChild(newScript, oldScript);
        });
    }
}

export const templateLoader = TemplateLoader.getInstance();
