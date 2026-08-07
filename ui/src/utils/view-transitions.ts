/**
 * View Transitions API helper.
 * Provides graceful fallback for browsers that don't support it.
 *
 * Usage:
 *   if (supportsViewTransitions()) {
 *     await withViewTransition(() => {
 *       // DOM changes here
 *     });
 *   } else {
 *     // Fallback: just apply changes directly
 *   }
 */

export function supportsViewTransitions(): boolean {
  return 'startViewTransition' in document;
}

/**
 * Execute DOM changes within a View Transition.
 * Falls back to direct execution if not supported.
 */
export async function withViewTransition(
  update: () => void | Promise<void>,
): Promise<void> {
  if (!supportsViewTransitions()) {
    await update();
    return;
  }

  const transition = (document as any).startViewTransition(update);
  await transition.finished;
}

/**
 * Create a View Transition with custom animation.
 * Allows specifying the animation for old/new content.
 */
export async function withViewTransitionCustom(
  update: () => void | Promise<void>,
  options: {
    oldContentAnimation?: string;
    newContentAnimation?: string;
  } = {},
): Promise<void> {
  if (!supportsViewTransitions()) {
    await update();
    return;
  }

  const transition = (document as any).startViewTransition(update);

  const oldContent = document.querySelector('::view-transition-old(root)') as any;
  const newContent = document.querySelector('::view-transition-new(root)') as any;

  if (oldContent && options.oldContentAnimation) {
    oldContent.style.animation = options.oldContentAnimation;
  }
  if (newContent && options.newContentAnimation) {
    newContent.style.animation = options.newContentAnimation;
  }

  await transition.finished;
}

/**
 * Morph animation for dialog open.
 * The dialog scales and fades in from the trigger button.
 */
export const dialogOpenAnimation = {
  newContent: 'dialogIn 0.25s cubic-bezier(0.34, 1.56, 0.64, 1)',
  oldContent: 'dialogOut 0.15s ease-out',
};

export const dialogCloseAnimation = {
  newContent: 'dialogOut 0.15s ease-out',
  oldContent: 'dialogIn 0.25s cubic-bezier(0.34, 1.56, 0.64, 1)',
};
