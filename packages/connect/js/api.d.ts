/* eslint-disable */
export declare function version(): string
export declare function test_lcs(s1: string, s2: string): number
export declare function test_subset_sum(a: Array<number>, t: number): boolean

export declare function bucket(name: string): BucketApi

export interface InitializeOptions {
  model: string
  externalTypes?: Record<string, string>
}

export interface FragApi {
  /** True when both endpoints are literal integers (no bindings needed). */
  isComplete(): boolean
  eval(bindings?: Record<string, number>): Promise<number>
}

export interface TxApi {
  setData(name: string, value: unknown): Promise<void>
  eval(frag: FragApi, bindings: Record<string, number>): Promise<number>
}

export interface BucketApi {
  initialize(opts: InitializeOptions): Promise<void>
  prepareFrag(src: string): Promise<FragApi>
  transaction(fn: (tx: TxApi) => Promise<void> | void): Promise<void>
}

declare const dimArk: {
  version: typeof version
  test_lcs: typeof test_lcs
  test_subset_sum: typeof test_subset_sum
  bucket: typeof bucket
}

export default dimArk
