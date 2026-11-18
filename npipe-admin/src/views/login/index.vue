<script setup lang="ts">
import { reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'
import Button from '@/components/ui/Button.vue'
import Field from '@/components/ui/Field.vue'
import Input from '@/components/ui/Input.vue'
import LanguageSwitch from '@/components/LanguageSwitch.vue'
import ThemeSwitch from '@/components/ThemeSwitch.vue'

const { t } = useI18n()
const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()
const toast = useToastStore()

const loading = ref(false)
const errorMsg = ref('')
const form = reactive({ username: '', password: '' })
const errors = reactive({ username: '', password: '' })

function validate() {
  errors.username = form.username.trim() ? '' : t('login.validationUsername')
  errors.password = form.password ? '' : t('login.validationPassword')
  return !errors.username && !errors.password
}

function mapLoginError(code: number, msg: string, status?: number) {
  if (status === 429 || code === -5) {
    const secs = /(\\d+)/.exec(msg)?.[1]
    return secs ? t('login.locked', { secs }) : msg || t('login.error')
  }
  if (code === -3) return t('login.userDisabled')
  if (code === -4) return t('login.noAccess')
  if (code === -2) return t('login.error')
  return msg || t('login.error')
}

async function onSubmit() {
  if (loading.value) return
  errorMsg.value = ''
  if (!validate()) return
  loading.value = true
  try {
    const { ok, msg, code } = await authStore.login(form.username, form.password)
    if (ok) {
      toast.success(t('login.success'))
      const redirect = (route.query.redirect as string) || (authStore.isAdmin ? '/dashboard' : `/players/${authStore.currentUserId}`)
      await router.push(redirect)
    } else {
      errorMsg.value = mapLoginError(code, msg)
    }
  } catch (e: unknown) {
    const err = e as { response?: { status?: number; data?: { code?: number; msg?: string } } }
    if (err?.response) {
      errorMsg.value = mapLoginError(err.response.data?.code ?? 0, err.response.data?.msg ?? '', err.response.status)
    } else {
      errorMsg.value = t('login.networkError')
    }
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="relative min-h-full overflow-hidden bg-canvas">
    <div class="login-aurora" aria-hidden="true">
      <div class="login-orb login-orb-a" />
      <div class="login-orb login-orb-b" />
      <div class="login-orb login-orb-c" />
      <div class="login-grid" />
    </div>

    <div class="relative flex min-h-full flex-col">
      <div class="flex items-center justify-between px-5 py-4">
        <div class="flex items-center gap-2">
          <img src="/logo.svg" alt="" class="h-8 w-8 rounded-lg" />
          <span class="text-sm font-semibold tracking-wide">{{ t('layout.appName') }}</span>
        </div>
        <div class="flex items-center gap-2">
          <LanguageSwitch />
          <ThemeSwitch />
        </div>
      </div>

      <div class="flex flex-1 items-center justify-center px-4 pb-16">
        <div class="relative w-full max-w-[420px] rounded-2xl border border-line bg-surface/95 p-7 shadow-lift backdrop-blur-sm">
          <p class="ui-kicker">{{ t('login.brandNote') }}</p>
          <h1 class="mt-2 text-2xl font-semibold tracking-tight">{{ t('login.title') }}</h1>
          <p class="mt-1 text-sm text-muted">{{ t('login.subtitle') }}</p>

          <div v-if="errorMsg" class="mt-5 flex items-start gap-2 rounded-lg border border-[var(--danger)]/30 bg-[rgba(214,69,69,.08)] px-3 py-2 text-sm text-[var(--danger)]" role="alert">
            {{ errorMsg }}
          </div>

          <form class="mt-6 space-y-4" @submit.prevent="onSubmit">
            <Field :label="t('login.username')" :error="errors.username" required html-for="login-username">
              <Input id="login-username" v-model="form.username" name="username" autocomplete="username" :placeholder="t('login.usernamePlaceholder')" :invalid="!!errors.username" />
            </Field>
            <Field :label="t('login.password')" :error="errors.password" required html-for="login-password">
              <Input id="login-password" v-model="form.password" type="password" name="password" autocomplete="current-password" :placeholder="t('login.passwordPlaceholder')" :invalid="!!errors.password" @keyup.enter="onSubmit" />
            </Field>
            <Button type="submit" variant="primary" size="lg" block :loading="loading">
              {{ loading ? t('login.submitting') : t('login.submit') }}
            </Button>
          </form>
        </div>
      </div>
    </div>
  </div>
</template>
