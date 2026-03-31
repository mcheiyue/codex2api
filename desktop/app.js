const phaseEl = document.getElementById('phase')
const summaryEl = document.getElementById('summary')
const statusPillEl = document.getElementById('status-pill')
const statusLabelEl = document.getElementById('status-label')
const configPathEl = document.getElementById('config-path')
const dbPathEl = document.getElementById('db-path')
const logPathEl = document.getElementById('log-path')
const adminUrlEl = document.getElementById('admin-url')
const apiUrlEl = document.getElementById('api-url')
const actionBarEl = document.getElementById('action-bar')
const enterAdminBtnEl = document.getElementById('enter-admin-btn')
const spinnerEl = document.getElementById('spinner')
const mainTitleEl = document.getElementById('main-title')

let adminUrl = 'http://127.0.0.1:8080/admin/'

function pushTimelineEntry(detail) {
  const timelineEl = document.getElementById('timeline')
  const timelineEntries = timelineEl.querySelectorAll('.timeline-item')
  timelineEntries.forEach(el => el.classList.remove('is-active'))

  const index = timelineEntries.length
  const title = detail.title ?? `阶段 ${index + 1}`
  const description = detail.message ?? ''
  const item = document.createElement('article')
  item.className = 'timeline-item is-active'
  item.innerHTML = `
    <span class="timeline-step">${index + 1}</span>
    <div>
      <strong>${title}</strong>
      <p>${description}</p>
    </div>
  `
  timelineEl.appendChild(item)
}

function applyStatus(detail) {
  if (!detail) return

  if (typeof detail.phase === 'string' && detail.phase) {
    phaseEl.textContent = detail.phase
  }

  if (typeof detail.message === 'string' && detail.message) {
    summaryEl.textContent = detail.message
  }

  if (typeof detail.label === 'string' && detail.label) {
    statusLabelEl.textContent = detail.label
  }

  if (typeof detail.level === 'string' && detail.level) {
    statusPillEl.dataset.level = detail.level
  }

  if (typeof detail.configPath === 'string' && detail.configPath) {
    configPathEl.textContent = detail.configPath
  }

  if (typeof detail.dbPath === 'string' && detail.dbPath) {
    dbPathEl.textContent = detail.dbPath
  }

  if (typeof detail.logPath === 'string' && detail.logPath) {
    logPathEl.textContent = detail.logPath
  }

  if (typeof detail.adminUrl === 'string' && detail.adminUrl) {
    adminUrl = detail.adminUrl
    adminUrlEl.textContent = detail.adminUrl
    const portMatch = detail.adminUrl.match(/:(\d+)\//)
    if (portMatch) {
      apiUrlEl.textContent = `http://127.0.0.1:${portMatch[1]}/v1/chat/completions`
    }
  }

  if (detail.level === 'ready') {
    spinnerEl.style.display = 'none'
    actionBarEl.style.display = 'flex'
    mainTitleEl.textContent = 'Codex2API 服务已就绪'
  }

  pushTimelineEntry(detail)
}

window.addEventListener('desktop-status', (event) => {
  applyStatus(event.detail)
})

enterAdminBtnEl.addEventListener('click', () => {
  window.location.replace(adminUrl)
})