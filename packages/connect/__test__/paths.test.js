const { describe, it } = require('node:test')
const assert = require('node:assert/strict')
const dimArk = require('../js/api.js')

const PATHS_MODEL = `
	  extern type Id
	  extern type Value
	  
	  data input: {values: {[Id]: Value}, edges: {[Id]: [Id]}}
	  
		paths = dp {
			node {
				key: Id
				payload = input.values[key]
				next = || {
					for to in input.edges[key] {
						yield node(to)	
					}	
				}
				
				combine = |a, b| a + b
				extend  = |a, b| a * b
				unit    = 1 
				zero = 0
			}
		}
	`

describe('paths.between', () => {
  it('computes between(1, 9) === 3600', async () => {
    const bucket = dimArk.bucket('paths_test')
    await bucket.initialize({
      model: PATHS_MODEL,
      externalTypes: { Id: 'u52', Value: 'u52' },
    })
    await bucket.transaction(async (tx) => {
      await tx.setData('input', {
        values: new Map([
          [1, 1],
          [2, 2],
          [3, 3],
          [4, 4],
          [5, 5],
          [6, 6],
          [7, 7],
          [8, 8],
          [9, 9],
        ]),
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
    })

    const frag1 = await bucket.prepareFrag(
      'paths.between(paths.node($begin), paths.node($end))',
    )
    const frag1Res = await frag1.eval({ begin: 1, end: 9 })
    assert.equal(frag1Res, 3600)
  })
})
