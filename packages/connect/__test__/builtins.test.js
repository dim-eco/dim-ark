const { describe, it } = require('node:test')
const assert = require('node:assert/strict')
const { test_subset_sum, test_lcs } = require('../index.js')

describe('test_subset_sum', () => {
  it('finds a subset that sums to the target', () => {
    assert.equal(test_subset_sum([1, 5, 2, 3, 6, 7], 14), true)
  })

  it('returns false when no subset reaches the target', () => {
    assert.equal(test_subset_sum([1, 2, 3], 100), false)
  })
})

describe('test_lcs', () => {
  it('returns the LCS length', () => {
    assert.equal(test_lcs('ABCD', 'BDED'), 2)
  })

  it('handles SIECIE / ISCISE case', () => {
    assert.equal(test_lcs('SIECIE', 'ISCISE'), 4)
  })
})
