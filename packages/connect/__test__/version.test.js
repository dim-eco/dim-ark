const { describe, it } = require('node:test')
const assert = require('node:assert/strict')
const { version: packageVersion } = require('../package.json')
const { version } = require('../index.js')

describe('version', () => {
  it('returns a non-empty string', () => {
    const result = version()
    assert.equal(typeof result, 'string')
    assert.ok(result.length > 0)
  })

  it('matches the package version', () => {
    assert.equal(version(), packageVersion)
  })
})
