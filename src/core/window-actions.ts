import { getPlatformWindow, type PlatformWindow } from './platform';

export type WindowActions = Record<string, unknown>;

type WindowActionRegistry = Record<string, WindowActions>;

export type WindowActionOptions = {
  namespace?: string;
  target?: PlatformWindow;
};

type WindowActionTarget = PlatformWindow & {
  __windowActionNamespaces__?: Record<string, WindowActions>;
  __windowActionRegistry__?: WindowActionRegistry;
};

function resolveOptions(options?: WindowActionOptions | PlatformWindow): Required<WindowActionOptions> {
  if (!options || 'document' in options || 'location' in options) {
    return { namespace: '', target: (options as PlatformWindow | undefined) ?? getPlatformWindow() };
  }

  return {
    namespace: options.namespace ?? '',
    target: options.target ?? getPlatformWindow(),
  };
}

function getRegistry(target: WindowActionTarget): WindowActionRegistry {
  if (!target.__windowActionRegistry__) {
    target.__windowActionRegistry__ = {};
  }

  return target.__windowActionRegistry__;
}

function getNamespacedActions(target: WindowActionTarget, namespace: string): WindowActions {
  if (!target.__windowActionNamespaces__) {
    target.__windowActionNamespaces__ = {};
  }

  if (!target.__windowActionNamespaces__[namespace]) {
    target.__windowActionNamespaces__[namespace] = {};
  }

  return target.__windowActionNamespaces__[namespace];
}

export function registerWindowActions(
  actions: WindowActions,
  options?: WindowActionOptions | PlatformWindow,
): PlatformWindow {
  const { namespace, target } = resolveOptions(options);
  const actionTarget = target as WindowActionTarget;

  Object.assign(actionTarget, actions);

  if (namespace) {
    Object.assign(getNamespacedActions(actionTarget, namespace), actions);
    getRegistry(actionTarget)[namespace] = {
      ...(getRegistry(actionTarget)[namespace] ?? {}),
      ...actions,
    };
  }

  return target;
}

export function unregisterWindowActions(
  actions: WindowActions,
  options?: WindowActionOptions | PlatformWindow,
): PlatformWindow {
  const { namespace, target } = resolveOptions(options);
  const actionTarget = target as WindowActionTarget;

  Object.entries(actions).forEach(([name, action]) => {
    if (actionTarget[name] === action) {
      delete actionTarget[name];
    }
  });

  if (namespace && actionTarget.__windowActionNamespaces__?.[namespace]) {
    const namespacedActions = actionTarget.__windowActionNamespaces__[namespace];
    Object.entries(actions).forEach(([name, action]) => {
      if (namespacedActions[name] === action) {
        delete namespacedActions[name];
      }
    });

    if (Object.keys(namespacedActions).length === 0) {
      delete actionTarget.__windowActionNamespaces__[namespace];
    }

    if (actionTarget.__windowActionRegistry__?.[namespace]) {
      delete actionTarget.__windowActionRegistry__[namespace];
    }
  }

  return target;
}
