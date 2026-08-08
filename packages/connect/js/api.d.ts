/* eslint-disable */
export declare function version(): string
export declare function test_lcs(s1: string, s2: string): number
export declare function test_subset_sum(a: Array<number>, t: number): boolean

export declare function bucket(name: string): BucketApi

export interface InitializeOptions {
  model: string
  externalTypes?: Record<string, string>
}

export interface BetweenNodeDebug {
  id: string
  value: number
  sum: number
  dp: number
}

export interface BetweenEdgeDebug {
  from: string
  to: string
}

/** Scalar between result plus filtered subgraph DP values from the engine. */
export interface BetweenDebug {
  result: number
  nodes: BetweenNodeDebug[]
  edges: BetweenEdgeDebug[]
}

export interface FragApi {
  /** True when both endpoints are fully literal (no bindings needed). */
  isComplete(): boolean
  eval(bindings?: Record<string, number>): Promise<number>
  /** Evaluate and return per-node DP debug info for visualization. */
  evalDebug(bindings?: Record<string, number>): Promise<BetweenDebug>
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
