<template>
  <div class="login-page">
    <!-- Animated background particles -->
    <div class="bg-particles">
      <div v-for="i in 20" :key="i" class="particle" :style="particleStyle(i)" />
    </div>

    <div class="login-card fade-in-up">
      <div class="login-header">
        <div class="brand-icon">⚡</div>
        <h1 class="brand-name">{{ $t('login.title') }}</h1>
        <p class="brand-sub">{{ $t('login.subtitle') }}</p>
      </div>

      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        @submit.prevent="onSubmit"
        class="login-form"
        size="large"
      >
        <el-form-item prop="username">
          <el-input
            v-model="form.username"
            :placeholder="$t('login.usernamePlaceholder')"
            :prefix-icon="User"
            autofocus
            @keyup.enter="onSubmit"
          />
        </el-form-item>

        <el-form-item prop="password">
          <el-input
            v-model="form.password"
            type="password"
            :placeholder="$t('login.passwordPlaceholder')"
            :prefix-icon="Lock"
            show-password
            @keyup.enter="onSubmit"
          />
        </el-form-item>

        <div class="remember-row">
          <el-checkbox v-model="autoLogin">{{ $t('login.autoLogin') }}</el-checkbox>
        </div>

        <el-button
          type="primary"
          class="login-btn"
          :loading="loading"
          native-type="submit"
          @click="onSubmit"
        >
          {{ loading ? $t('login.submitting') : $t('login.submit') }}
        </el-button>

        <!-- Inline error message -->
        <transition name="err-slide">
          <div v-if="errorMsg" class="login-error">
            <el-icon><WarningFilled /></el-icon>
            {{ errorMsg }}
          </div>
        </transition>
      </el-form>

      <p class="login-footer">npipe &copy; {{ new Date().getFullYear() }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { User, Lock, WarningFilled } from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'

const { t } = useI18n()
const router = useRouter()
const route  = useRoute()
const authStore = useAuthStore()

const formRef   = ref<FormInstance>()
const loading   = ref(false)
const errorMsg  = ref('')
const autoLogin = ref(localStorage.getItem('autoLogin') === 'true')

const form = reactive({
  username: localStorage.getItem('savedUser') ?? '',
  password: '',
})

const rules: FormRules = {
  username: [{ required: true, message: () => t('login.validationUsername'), trigger: 'blur' }],
  password: [{ required: true, message: () => t('login.validationPassword'), trigger: 'blur' }],
}

async function onSubmit() {
  errorMsg.value = ''
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return

  loading.value = true
  try {
    const { ok, msg } = await authStore.login(form.username, form.password)
    if (ok) {
      if (autoLogin.value) {
        localStorage.setItem('autoLogin', 'true')
        localStorage.setItem('savedUser', form.username)
      } else {
        localStorage.removeItem('autoLogin')
        localStorage.removeItem('savedUser')
      }
      ElMessage.success(t('login.success'))
      const redirect = (route.query.redirect as string) ?? '/dashboard'
      router.push(redirect)
    } else {
      errorMsg.value = msg || t('login.error')
    }
  } catch {
    // errors handled by interceptor
  } finally {
    loading.value = false
  }
}

// Random particle style
function particleStyle(i: number) {
  const size = 4 + (i % 5) * 3
  return {
    width:  `${size}px`,
    height: `${size}px`,
    left:   `${(i * 17 + 5) % 100}%`,
    top:    `${(i * 23 + 10) % 100}%`,
    animationDelay:    `${(i * 0.4) % 4}s`,
    animationDuration: `${6 + (i % 4) * 2}s`,
    opacity:           `${0.1 + (i % 5) * 0.05}`,
  }
}
</script>

<style scoped lang="scss">
.login-page {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(145deg, #0a0e1a 0%, #0d1428 40%, #0e1530 70%, #0a0e1a 100%);
  position: relative;
  overflow: hidden;
}

.bg-particles {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.particle {
  position: absolute;
  border-radius: 50%;
  background: #5b8ff9;
  animation: float linear infinite;
}

@keyframes float {
  0%   { transform: translateY(0) scale(1); }
  50%  { transform: translateY(-36px) scale(1.08); }
  100% { transform: translateY(0) scale(1); }
}

.login-card {
  position: relative;
  z-index: 2;
  width: 400px;
  background: rgba(24, 29, 46, 0.88);
  backdrop-filter: blur(24px);
  border: 1px solid rgba(91, 143, 249, 0.18);
  border-radius: 18px;
  padding: 44px 40px 36px;
  box-shadow: 0 24px 64px rgba(0,0,0,.6), 0 0 0 1px rgba(91,143,249,.06);
}

.login-header {
  text-align: center;
  margin-bottom: 32px;

  .brand-icon {
    font-size: 40px;
    display: block;
    margin-bottom: 10px;
    filter: drop-shadow(0 0 14px rgba(91,143,249,.7));
  }

  .brand-name {
    font-size: 22px;
    font-weight: 700;
    color: #dde3f4;
    margin: 0 0 6px;
    letter-spacing: .5px;
  }

  .brand-sub {
    font-size: 13px;
    color: rgba(139,147,176,.8);
    margin: 0;
  }
}

.login-form {
  :deep(.el-input__wrapper) {
    background: rgba(16, 19, 40, 0.9) !important;
    box-shadow: 0 0 0 1px rgba(39,45,68,.9) inset !important;
    border-radius: 8px;
    &:hover  { box-shadow: 0 0 0 1px rgba(91,143,249,.7) inset !important; }
    &.is-focus { box-shadow: 0 0 0 1.5px #5b8ff9 inset !important; }
  }
  :deep(.el-input__inner) {
    color: #dde3f4;
    &::placeholder { color: rgba(139,147,176,.6); }
  }
  :deep(.el-input__prefix-icon) { color: rgba(139,147,176,.7); }
}

.remember-row {
  display: flex;
  justify-content: space-between;
  margin-bottom: 20px;
  margin-top: -4px;
  :deep(.el-checkbox__label) { color: #8b93b0; font-size: 13px; }
}

.login-btn {
  width: 100%;
  height: 44px;
  font-size: 15px;
  font-weight: 600;
  border-radius: 9px;
  letter-spacing: 2px;
  background: linear-gradient(90deg, #3b6de8, #5b8ff9);
  border: none;
  box-shadow: 0 4px 16px rgba(59,109,232,.4);
  transition: all .2s;

  &:hover {
    box-shadow: 0 6px 22px rgba(59,109,232,.55);
    background: linear-gradient(90deg, #4678f0, #6d9dfb);
  }
  &:active { transform: translateY(1px); }
}

.login-footer {
  text-align: center;
  color: rgba(139, 148, 158, 0.5);
  font-size: 12px;
  margin: 20px 0 0;
}

.login-error {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 12px;
  padding: 10px 14px;
  border-radius: 8px;
  background: rgba(240, 96, 96, 0.12);
  border: 1px solid rgba(240, 96, 96, 0.35);
  color: #f06060;
  font-size: 13px;
  font-weight: 500;
}

.err-slide-enter-active, .err-slide-leave-active {
  transition: opacity .25s ease, transform .25s ease;
}
.err-slide-enter-from, .err-slide-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

</style>

