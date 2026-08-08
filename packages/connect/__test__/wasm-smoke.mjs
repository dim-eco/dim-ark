import { createRequire } from 'node:module'
import assert from 'node:assert/strict'

const require = createRequire(import.meta.url)
const { bucket, version } = require('../dim-ark-connect.wasi.cjs')

const model = `
  extern type Id
  extern type Value
  data input: {values: {[Id]: Value}, edges: {[Id]: [Id]}}
  paths = dp {
    node {
      key: Id
      payload = input.values[key]
      next = || { for to in input.edges[key] { yield node(to) } }
      combine = |a, b| a + b
      extend  = |a, b| a * b
      unit = 1
      zero = 0
    }
  }
`

const b = bucket('wasm_smoke')
b.initialize({ model, externalTypes: { Id: 'u52', Value: 'u52' } })
b.setData('input', {
  values: { 1: 1, 2: 2, 3: 3, 4: 4, 5: 5, 6: 6, 7: 7, 8: 8, 9: 9 },
  edges: {
    1: [2, 3],
    2: [4, 5],
    3: [5, 6],
    4: [7],
    5: [7, 9],
    6: [8],
    7: [9],
    8: [9],
    9: [],
  },
})
const frag = b.prepareFrag('paths.between(paths.node($begin), paths.node($end))')
const result = frag.eval({ begin: 1, end: 9 })
assert.equal(Number(result), 3600)
console.log('wasm smoke ok', version(), result)
