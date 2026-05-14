import type { Mock } from '@vitest/spy';

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
