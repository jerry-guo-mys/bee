import path from 'path'

import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig, loadEnv } from 'vite'

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  /** REST 管理接口：仅 bee-web 提供 /api/*；bee-gateway 无这些路由 */
  const apiTarget = env.VITE_DEV_API_PROXY_TARGET || 'http://127.0.0.1:8080'
  /** WebSocket：可指向 bee-gateway（默认 9000）或 bee-web */
  const wsTarget = env.VITE_DEV_WS_PROXY_TARGET || 'ws://127.0.0.1:8080'

  return {
    plugins: [react(), tailwindcss()],
    resolve: {
      alias: {
        '@': path.resolve(__dirname, './src'),
      },
    },
    server: {
      port: 3000,
      proxy: {
        '/api': {
          target: apiTarget,
          changeOrigin: true,
        },
        '/ws': {
          target: wsTarget,
          ws: true,
        },
      },
    },
  }
})
