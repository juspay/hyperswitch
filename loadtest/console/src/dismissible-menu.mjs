import {tick} from 'svelte';

/** Light-dismiss behavior for native details used as action menus. */
/** @param {HTMLDetailsElement} node */
export function dismissibleMenu(node) {
  const document = node.ownerDocument;
  const summary = node.querySelector('summary');

  /** @param {Event} event */
  function dismissOutside(event) {
    if (node.open && !event.composedPath().includes(node)) node.open = false;
  }

  /** @param {KeyboardEvent} event */
  function onKeydown(event) {
    if (!node.open || event.key !== 'Escape') return;
    event.preventDefault();
    event.stopPropagation();
    node.open = false;
    summary?.focus();
  }

  /** @param {MouseEvent} event */
  function onClick(event) {
    dismissOutside(event);
    if (!node.open || !(event.target instanceof Element)) return;
    const action = event.target.closest('button, a[href]');
    if (!action || !node.contains(action) || action.matches(':disabled, [aria-disabled="true"]')) return;
    const hadFocus = node.contains(document.activeElement);
    // Let the action finish (including keyed row moves) before hiding its focused button.
    queueMicrotask(async () => {
      await tick();
      if (!node.isConnected) return;
      const restoreFocus = hadFocus && (document.activeElement === document.body || node.contains(document.activeElement));
      node.open = false;
      if (restoreFocus) summary?.focus();
    });
  }

  document.addEventListener('pointerdown', dismissOutside, true);
  document.addEventListener('focusin', dismissOutside, true);
  document.addEventListener('click', onClick, true);
  document.addEventListener('keydown', onKeydown, true);
  return {
    destroy() {
      document.removeEventListener('pointerdown', dismissOutside, true);
      document.removeEventListener('focusin', dismissOutside, true);
      document.removeEventListener('click', onClick, true);
      document.removeEventListener('keydown', onKeydown, true);
    }
  };
}
