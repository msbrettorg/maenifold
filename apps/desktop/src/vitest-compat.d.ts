import type { Mock } from '@vitest/spy';
import type { DOMWrapper } from '@vue/test-utils';

declare module '@vitest/spy' {
  export function fn(
    originalImplementation?: (...args: any[]) => any
  ): Mock<(...args: any[]) => any>;
  export function fn<
    TArgs extends any[],
    TReturn
  >(
    originalImplementation?: (...args: TArgs) => TReturn
  ): Mock<(...args: TArgs) => TReturn>;
  export function fn<T extends (...args: any[]) => any = (...args: any[]) => any>(
    originalImplementation?: T
  ): Mock<T>;
}

declare module '@vue/test-utils' {
  interface VueWrapper {
    get<T extends Element = Element>(selector: string): DOMWrapper<T>;
  }
}
