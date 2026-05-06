/**
 * 事件总线 - 用于模块间通信
 * 使用发布-订阅模式
 */

type EventHandler = (data?: any) => void;

class EventBus {
  private events: Map<string, EventHandler[]> = new Map();

  /**
   * 订阅事件
   */
  on(event: string, handler: EventHandler): void {
    if (!this.events.has(event)) {
      this.events.set(event, []);
    }
    this.events.get(event)!.push(handler);
  }

  /**
   * 取消订阅
   */
  off(event: string, handler: EventHandler): void {
    const handlers = this.events.get(event);
    if (handlers) {
      const index = handlers.indexOf(handler);
      if (index > -1) {
        handlers.splice(index, 1);
      }
    }
  }

  /**
   * 发布事件
   */
  emit(event: string, data?: any): void {
    const handlers = this.events.get(event);
    if (handlers) {
      handlers.forEach(handler => handler(data));
    }
  }

  /**
   * 一次性订阅
   */
  once(event: string, handler: EventHandler): void {
    const onceHandler: EventHandler = (data) => {
      handler(data);
      this.off(event, onceHandler);
    };
    this.on(event, onceHandler);
  }
}

// 导出单例
export const eventBus = new EventBus();
