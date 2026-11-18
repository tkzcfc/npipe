import { describe, expect, it } from 'vitest'
import {
  buildTunnelRequest,
  canManageTunnel,
  clampTunnelEndpoints,
  defaultTunnelForm,
  prefillEndpointsForUser,
  tunnelDisplayName,
} from '../src/utils/tunnel'

const admin = { isAdmin: true, currentUserId: 0 }
const user = { isAdmin: false, currentUserId: 42 }

describe('canManageTunnel', () => {
  it('allows admins for any endpoints', () => {
    expect(canManageTunnel({ sender: 1, receiver: 2 }, admin)).toBe(true)
  })

  it('allows a user only when they are the sender and the receiver is self or the server', () => {
    expect(canManageTunnel({ sender: 42, receiver: 0 }, user)).toBe(true)
    expect(canManageTunnel({ sender: 42, receiver: 42 }, user)).toBe(true)
    expect(canManageTunnel({ sender: 42, receiver: 7 }, user)).toBe(false)
    expect(canManageTunnel({ sender: 7, receiver: 42 }, user)).toBe(false)
    expect(canManageTunnel({ sender: 0, receiver: 0 }, user)).toBe(false)
  })
})

describe('prefillEndpointsForUser', () => {
  it('lets an admin prefill the selected user as sender and the server as receiver', () => {
    expect(prefillEndpointsForUser(88, admin)).toEqual({ sender: 88, receiver: 0 })
  })

  it('does not let a regular user create a tunnel as someone else', () => {
    expect(prefillEndpointsForUser(88, user)).toEqual({ sender: 42, receiver: 0 })
    expect(prefillEndpointsForUser(42, user)).toEqual({ sender: 42, receiver: 0 })
  })
})

describe('clampTunnelEndpoints', () => {
  it('forces non-admin requests back to the current user', () => {
    expect(clampTunnelEndpoints(99, 8, user)).toEqual({ sender: 42, receiver: 0 })
    expect(clampTunnelEndpoints(99, 42, user)).toEqual({ sender: 42, receiver: 42 })
    expect(clampTunnelEndpoints(1, 2, admin)).toEqual({ sender: 1, receiver: 2 })
  })
})

describe('buildTunnelRequest', () => {
  it('clears the endpoint for proxy types', () => {
    const form = defaultTunnelForm({ sender: 1, receiver: 0 })
    form.source = '127.0.0.1:9000'
    form.endpoint = 'should-ignore'
    form.tunnel_type = 2
    const req = buildTunnelRequest(form, admin)
    expect(req.endpoint).toBe('')
    expect(req.sender).toBe(1)
  })
})

describe('tunnelDisplayName', () => {
  it('prefers the description and falls back to type plus source', () => {
    expect(tunnelDisplayName({ description: 'edge', source: '1.1.1.1:80', tunnel_type: 0 })).toBe('edge')
    expect(tunnelDisplayName({ description: '  ', source: '1.1.1.1:80', tunnel_type: 0 })).toBe('TCP 1.1.1.1:80')
  })
})
