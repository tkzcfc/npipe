import { describe, expect, it } from 'vitest'
import { messageFromResult, messageFromUnknown } from '../src/utils/apiMessage'

const t = (key: string) => (key === 'common.forbidden' ? 'NO_ACCESS' : key)

describe('api messages', () => {
  it('maps API code 403 and the raw Forbidden text to the i18n permission string', () => {
    expect(messageFromResult({ code: 403, msg: 'Forbidden' }, t, 'fallback')).toBe('NO_ACCESS')
    expect(messageFromResult({ code: 403 }, t, 'fallback')).toBe('NO_ACCESS')
    expect(messageFromResult({ code: -1, msg: 'user already exists' }, t, 'fallback')).toBe('user already exists')
    expect(messageFromResult({ code: 0 }, t, 'fallback')).toBe('fallback')
  })

  it('maps HTTP 403 errors the same way', () => {
    expect(messageFromUnknown({ response: { status: 403, data: {} } }, t, 'fallback')).toBe('NO_ACCESS')
  })
})
