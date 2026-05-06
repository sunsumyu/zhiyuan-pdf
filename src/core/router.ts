/**
 * 路由管理器 - 单例模式
 * 负责页面导航和 URL hash 管理
 */

export class Router {
  private static instance: Router;
  private currentRoute: string = '';
  private routes: Map<string, () => void> = new Map();

  private constructor() {
    this.initializeRouting();
  }

  public static getInstance(): Router {
    if (!Router.instance) {
      Router.instance = new Router();
    }
    return Router.instance;
  }

  /**
   * 注册路由
   */
  public registerRoute(path: string, handler: () => void): void {
    this.routes.set(path, handler);
  }

  /**
   * 导航到指定路由
   */
  public navigateTo(path: string): void {
    console.log(`[Router] Navigating to: ${path}`);
    this.currentRoute = path;
    
    // 更新 URL hash
    if (path) {
      window.location.hash = `#${path}`;
    } else {
      window.location.hash = '';
    }

    // 执行路由处理器
    const handler = this.routes.get(path);
    if (handler) {
      handler();
    }
  }

  /**
   * 返回上一页
   */
  public goBack(): void {
    console.log('[Router] Going back');
    this.navigateTo('');
  }

  /**
   * 获取当前路由
   */
  public getCurrentRoute(): string {
    return this.currentRoute;
  }

  /**
   * 初始化路由系统
   */
  private initializeRouting(): void {
    // 监听 hash 变化（浏览器前进后退）
    window.addEventListener('hashchange', () => {
      this.handleHashChange();
    });

    // 页面加载时恢复路由
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', () => {
        this.handleHashChange();
      });
    } else {
      this.handleHashChange();
    }
  }

  /**
   * 处理 hash 变化
   */
  private handleHashChange(): void {
    const hash = window.location.hash;
    
    if (!hash || hash === '#') {
      // 返回首页
      const handler = this.routes.get('');
      if (handler) {
        this.currentRoute = '';
        handler();
      }
    } else {
      // 移除 # 符号
      const path = hash.substring(1);
      const handler = this.routes.get(path);
      if (handler) {
        this.currentRoute = path;
        handler();
      }
    }
  }
}

// 导出单例实例
export const router = Router.getInstance();
