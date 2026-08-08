'use strict'

const native = require('../index.js')

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
    eval(bindings) {
      return Promise.resolve(frag.eval(bindings))
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

module.exports = {
  version: native.version,
  test_lcs: native.test_lcs,
  test_subset_sum: native.test_subset_sum,
  bucket: (name) => wrapBucket(native.bucket(name)),
  Bucket: native.Bucket,
  Frag: native.Frag,
}
