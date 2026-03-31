const phaseEl = document.getElementById('phase')
const summaryEl = document.getElementById('summary')
const statusPillEl = document.getElementById('status-pill')
const statusLabelEl = document.getElementById('status-label')
const runtimeDirEl = document.getElementById('runtime-dir')
const logPathEl = document.getElementById('log-path')
const adminUrlEl = document.getElementById('admin-url')
const timelineEl = document.getElementById('timeline')

const timelineEntries = []

function pushTimelineEntry(detail) {
  timelineEntries.push(detail)
  timelineEl.innerHTML = timelineEntries
    .map((entry, index) => {
      const title = entry.title ?? `阶段 ${index + 1}`
      const description = entry.message ?? ''
      const activeClass = index === timelineEntries.length - 1 ? ' is-active' : ''
      return `
        <article class="timeline-item${activeClass}">
          <span class="timeline-step">${index + 1}</span>
          <div>
            <strong>${title}</strong>
            <p>${description}</p>
          </div>
        </article>
      `
    })
    .join('')
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

  if (typeof detail.runtimeDir === 'string' && detail.runtimeDir) {
    runtimeDirEl.textContent = detail.runtimeDir
  }

  if (typeof detail.logPath === 'string' && detail.logPath) {
    logPathEl.textContent = detail.logPath
  }

  if (typeof detail.adminUrl === 'string' && detail.adminUrl) {
    adminUrlEl.textContent = detail.adminUrl
  }

  pushTimelineEntry(detail)
}

window.addEventListener('desktop-status', (event) => {
  applyStatus(event.detail)
})
