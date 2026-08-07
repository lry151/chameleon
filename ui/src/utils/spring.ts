/**
 * Spring physics for natural motion.
 * Lightweight solver, no external dependencies.
 *
 * Usage:
 *   const spring = createSpring({ from: 0, to: 1, stiffness: 300, damping: 20 });
 *   spring.onUpdate = (value) => { element.style.transform = `scale(${value})`; };
 *   spring.start();
 *
 * Config:
 *   stiffness: 100-500 (higher = snappier)
 *   damping: 10-40 (higher = less bounce)
 *   mass: 0.5-2 (default 1)
 */

export interface SpringConfig {
  from: number;
  to: number;
  stiffness?: number;
  damping?: number;
  mass?: number;
  precision?: number;
}

export interface Spring {
  start: () => void;
  stop: () => void;
  onUpdate?: (value: number) => void;
  onDone?: () => void;
}

export function createSpring(config: SpringConfig): Spring {
  const {
    from,
    to,
    stiffness = 300,
    damping = 20,
    mass = 1,
    precision = 0.001,
  } = config;

  let position = from;
  let velocity = 0;
  let rafId: number | null = null;
  let lastTime: number | null = null;

  const spring: Spring = {
    onUpdate: undefined,
    onDone: undefined,

    start() {
      if (rafId !== null) return;
      lastTime = null;
      const step = (timestamp: number) => {
        if (lastTime === null) {
          lastTime = timestamp;
          rafId = requestAnimationFrame(step);
          return;
        }

        const deltaTime = Math.min((timestamp - lastTime) / 1000, 0.064);
        lastTime = timestamp;

        const springForce = -stiffness * (position - to);
        const dampingForce = -damping * velocity;
        const acceleration = (springForce + dampingForce) / mass;

        velocity += acceleration * deltaTime;
        position += velocity * deltaTime;

        spring.onUpdate?.(position);

        if (
          Math.abs(position - to) < precision &&
          Math.abs(velocity) < precision
        ) {
          spring.onDone?.();
          rafId = null;
          return;
        }

        rafId = requestAnimationFrame(step);
      };

      rafId = requestAnimationFrame(step);
    },

    stop() {
      if (rafId !== null) {
        cancelAnimationFrame(rafId);
        rafId = null;
      }
    },
  };

  return spring;
}

/**
 * Animate a value with spring physics.
 * Returns a Promise that resolves when the animation completes.
 */
export function springTo(
  target: { [key: string]: number },
  config: Omit<SpringConfig, 'from' | 'to'>,
  onUpdate: (values: { [key: string]: number }) => void,
): Promise<void> {
  return new Promise((resolve) => {
    const keys = Object.keys(target);
    const springs = keys.map((key) => {
      const spring = createSpring({
        ...config,
        from: 0,
        to: target[key],
      });
      return { key, spring, current: 0 };
    });

    let doneCount = 0;
    springs.forEach(({ spring, key }) => {
      spring.onUpdate = (value) => {
        const idx = springs.findIndex((s) => s.key === key);
        springs[idx].current = value;
        const values: { [key: string]: number } = {};
        springs.forEach(({ key, current }) => {
          values[key] = current;
        });
        onUpdate(values);
      };
      spring.onDone = () => {
        doneCount++;
        if (doneCount === springs.length) {
          resolve();
        }
      };
      spring.start();
    });
  });
}

/**
 * Staggered spring animation for lists.
 */
export function staggerSprings(
  elements: HTMLElement[],
  config: Omit<SpringConfig, 'from' | 'to'> & { to: number },
  apply: (el: HTMLElement, value: number) => void,
  staggerDelay = 30,
): Promise<void> {
  return new Promise<void>((resolve) => {
    let doneCount = 0;
    elements.forEach((el, i) => {
      setTimeout(() => {
        const spring = createSpring({
          ...config,
          from: 0,
        });
        spring.onUpdate = (value) => {
          apply(el, value);
        };
        spring.onDone = () => {
          apply(el, config.to);
          doneCount++;
          if (doneCount === elements.length) {
            resolve();
          }
        };
        spring.start();
      }, i * staggerDelay);
    });
  });
}
