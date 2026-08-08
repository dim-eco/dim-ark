import * as native from '../dim-ark-connect.wasi-browser.js'

function toPlain(value) {
  if (value instanceof Map) {
    const out = Object.create(null)
    for (const [k, v] of value) {
      out[k] = toPlain(v)
    }
    return out
  }
  if (Array.isArray(value)) {
    return value.map(toPlain)
  }
  if (value !== null && typeof value === 'object') {
    const out = Object.create(null)
    for (const [k, v] of Object.entries(value)) {
      out[k] = toPlain(v)
    }
    return out
  }
  return value
}

function wrapFrag(frag) {
  return {
    isComplete() {
      return typeof frag.isComplete === 'function' ? frag.isComplete() : false
    },
    eval(bindings) {
      return Promise.resolve(frag.eval(bindings ?? {}))
    },
    evalDebug(bindings) {
      const fn = frag.evalDebug
      if (typeof fn !== 'function') {
        return Promise.reject(
          new Error(
            'Frag.evalDebug missing from WASM — run `npm run build:wasm` in packages/connect and hard-refresh dimviz',
          ),
        )
      }
      return Promise.resolve(fn.call(frag, bindings ?? {}))
    },
  }
}

function wrapBucket(bucket) {
  return {
    initialize(opts) {
      return Promise.resolve(bucket.initialize(opts))
    },
    prepareFrag(src) {
      return Promise.resolve(wrapFrag(bucket.prepareFrag(src)))
    },
    transaction(fn) {
      const snap = bucket.snapshotEnv()
      const tx = {
        setData(name, value) {
          return Promise.resolve(bucket.setData(name, toPlain(value)))
        },
        eval(frag, bindings) {
          return frag.eval(bindings)
        },
      }
      return Promise.resolve()
        .then(() => fn(tx))
        .catch((err) => {
          bucket.restoreEnv(snap)
          throw err
        })
    },
  }
}

export const version = native.version
export const test_lcs = native.test_lcs
export const test_subset_sum = native.test_subset_sum
export const Bucket = native.Bucket
export const Frag = native.Frag
export function bucket(name) {
  return wrapBucket(native.bucket(name))
}

export default {
  version,
  test_lcs,
  test_subset_sum,
  bucket,
  Bucket,
  Frag,
}
