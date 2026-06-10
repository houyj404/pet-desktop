// ══════════════════════════════════════════════════════════════
// 桌面小宠物 — main.js
// 前端逻辑：设置、任务管理、宠物状态、右键菜单、拖拽、整点提醒
// ══════════════════════════════════════════════════════════════

import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => document.querySelectorAll(sel);

// ── 全局状态 ─────────────────────────────────────────────────
const app = {
  currentFilter: 'pending',
  restMode: false,
  muted: false,
  lastCheckedHour: -1,
  idleTimer: null,
  isSleeping: false,
  popupTimer: null,
};

let settings = {
  voice_enabled: 'true',
  voice_mode: 'tts',
  tts_volume: '80',
  tts_rate: '0',
  hourly_enabled: 'true',
  hourly_start_hour: '7',
  hourly_end_hour: '22',
  auto_start: 'true',
  pet_transparency: '100',
  edge_snap: 'true',
};

// ── DOM 缓存 ─────────────────────────────────────────────────
const DOM = {};
function cacheDom() {
  DOM.cat = $('#cat');
  DOM.bubble = $('#bubble');
  DOM.bubbleText = $('#bubble-text');
  DOM.statusIcon = $('#status-icon');
  DOM.petArea = $('#pet-area');

  DOM.taskPopup = $('#task-popup');
  DOM.popupTaskList = $('#popup-task-list');
  DOM.btnQuickAdd = $('#btn-quick-add');
  DOM.btnViewAll = $('#btn-view-all');

  DOM.taskModal = $('#task-modal');
  DOM.taskList = $('#task-list');
  DOM.btnAddTask = $('#btn-add-task');

  DOM.addModal = $('#add-modal');
  DOM.inpTitle = $('#inp-title');
  DOM.inpDue = $('#inp-due');
  DOM.inpDesc = $('#inp-desc');
  DOM.inpRemind = $('#inp-remind');
  DOM.btnSave = $('#btn-save');

  DOM.settingsModal = $('#settings-modal');
  DOM.setAutostart = $('#set-autostart');
  DOM.setEdgesnap = $('#set-edgesnap');
  DOM.setOpacity = $('#set-opacity');
  DOM.setHourly = $('#set-hourly');
  DOM.setHStart = $('#set-h-start');
  DOM.setHEnd = $('#set-h-end');
  DOM.setVoice = $('#set-voice');
  DOM.setVolume = $('#set-volume');

  DOM.ctxMenu = $('#ctx-menu');
}

// ═══════════════════════════════════════════════════════════════
// 1. 设置模块
// ═══════════════════════════════════════════════════════════════

async function loadSettings() {
  try {
    const list = await invoke('get_settings');
    const map = {};
    list.forEach((s) => (map[s.key] = s.value));
    Object.assign(settings, map);
    applySettingsToUI();
    applyTransparency();
  } catch (e) {
    console.error('loadSettings failed:', e);
  }
}

function applySettingsToUI() {
  DOM.setAutostart.checked = settings.auto_start === 'true';
  DOM.setEdgesnap.checked = settings.edge_snap === 'true';
  DOM.setOpacity.value = settings.pet_transparency;
  DOM.setHourly.checked = settings.hourly_enabled === 'true';
  DOM.setHStart.value = settings.hourly_start_hour;
  DOM.setHEnd.value = settings.hourly_end_hour;
  DOM.setVoice.checked = settings.voice_enabled === 'true';
  DOM.setVolume.value = settings.tts_volume;
}

function applyTransparency() {
  const val = parseInt(settings.pet_transparency, 10) / 100;
  document.body.style.opacity = val;
}

async function saveSetting(key, value) {
  settings[key] = String(value);
  try {
    await invoke('update_setting', { key, value: String(value) });
  } catch (e) {
    console.error('saveSetting failed:', e);
  }
}

function bindSettingsControls() {
  const pairs = [
    [DOM.setAutostart, 'auto_start'],
    [DOM.setEdgesnap, 'edge_snap'],
    [DOM.setOpacity, 'pet_transparency'],
    [DOM.setHourly, 'hourly_enabled'],
    [DOM.setHStart, 'hourly_start_hour'],
    [DOM.setHEnd, 'hourly_end_hour'],
    [DOM.setVoice, 'voice_enabled'],
    [DOM.setVolume, 'tts_volume'],
  ];

  pairs.forEach(([el, key]) => {
    el.addEventListener('change', () => {
      const val = el.type === 'checkbox' ? el.checked : el.value;
      saveSetting(key, val);
      if (key === 'pet_transparency') applyTransparency();
    });
  });
}

// ═══════════════════════════════════════════════════════════════
// 2. 宠物状态轮询 & CSS 切换
// ═══════════════════════════════════════════════════════════════

const PET_STATES = ['idle', 'remind', 'warning', 'sad', 'recover', 'happy', 'sleeping'];

async function pollPetState() {
  try {
    const info = await invoke('get_pet_state');
    applyPetState(info);
  } catch (e) {
    console.error('pollPetState failed:', e);
  }
}

function applyPetState(info) {
  const stateName = (info.state || 'IDLE').toLowerCase();
  const cat = DOM.cat;

  // 移除旧状态 class
  PET_STATES.forEach((s) => cat.classList.remove(`state-${s}`));
  cat.classList.add(`state-${stateName}`);

  // Zzz 装饰（sleeping 时显示）
  const zzz = cat.querySelector('.zzz');
  if (stateName === 'sleeping') {
    zzz.classList.remove('hidden');
    app.isSleeping = true;
  } else {
    zzz.classList.add('hidden');
    app.isSleeping = false;
  }

  // Sparkles 装饰（happy 时显示）
  const sparkles = cat.querySelector('.sparkles');
  if (stateName === 'happy') {
    sparkles.classList.remove('hidden');
  } else {
    sparkles.classList.add('hidden');
  }

  // recover 动画结束后自动回 idle
  if (stateName === 'recover') {
    setTimeout(() => {
      cat.classList.remove('state-recover');
      cat.classList.add('state-idle');
    }, 800);
  }
}

// ═══════════════════════════════════════════════════════════════
// 3. 气泡系统
// ═══════════════════════════════════════════════════════════════

let bubbleTimer = null;

function showBubble(text, duration = 3000) {
  DOM.bubbleText.textContent = text;
  DOM.bubble.classList.remove('hidden');
  clearTimeout(bubbleTimer);
  bubbleTimer = setTimeout(() => {
    DOM.bubble.classList.add('hidden');
  }, duration);
}

function hideBubble() {
  DOM.bubble.classList.add('hidden');
  clearTimeout(bubbleTimer);
}

// ═══════════════════════════════════════════════════════════════
// 4. 任务弹窗（悬浮）
// ═══════════════════════════════════════════════════════════════

async function loadPopupTasks() {
  try {
    const tasks = await invoke('get_tasks', { filter: 'pending' });
    DOM.popupTaskList.innerHTML = '';

    const show = tasks.slice(0, 3);
    show.forEach((t) => {
      const li = document.createElement('li');
      const urgent = isUrgent(t.due_time);
      li.innerHTML = `<span class="${urgent ? 'task-urgent' : 'task-ok'}">●</span> ${escapeHtml(t.title)} <small>${formatDueTime(t.due_time)}</small>`;
      DOM.popupTaskList.appendChild(li);
    });

    if (tasks.length === 0) {
      const li = document.createElement('li');
      li.textContent = '暂无待办事项 ✨';
      li.style.color = 'var(--text-light)';
      DOM.popupTaskList.appendChild(li);
    }
  } catch (e) {
    console.error('loadPopupTasks failed:', e);
  }
}

function isUrgent(dueTime) {
  if (!dueTime) return false;
  const due = new Date(dueTime.replace(' ', 'T'));
  const now = new Date();
  const diff = due - now;
  return diff > 0 && diff < 3600000; // < 1h
}

function isOverdue(dueTime) {
  if (!dueTime) return false;
  const due = new Date(dueTime.replace(' ', 'T'));
  return due < new Date();
}

function formatDueTime(dueTime) {
  if (!dueTime) return '';
  const due = new Date(dueTime.replace(' ', 'T'));
  const now = new Date();
  const diff = due - now;

  if (diff < 0) return '已过期';
  if (diff < 3600000) return `${Math.ceil(diff / 60000)}分钟`;
  if (diff < 86400000) return `${Math.ceil(diff / 3600000)}小时`;
  return due.toLocaleDateString('zh-CN', { month: 'numeric', day: 'numeric' });
}

function formatDateTime(str) {
  if (!str) return '';
  const d = new Date(str.replace(' ', 'T'));
  return d.toLocaleString('zh-CN', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
}

function escapeHtml(s) {
  const div = document.createElement('div');
  div.textContent = s;
  return div.innerHTML;
}

function bindPopupEvents() {
  let hoverTimer = null;

  DOM.cat.addEventListener('mouseenter', () => {
    hoverTimer = setTimeout(async () => {
      await loadPopupTasks();
      DOM.taskPopup.classList.remove('hidden');
    }, 300);
  });

  DOM.cat.addEventListener('mouseleave', () => {
    clearTimeout(hoverTimer);
    // 延迟隐藏，让鼠标可以移到弹窗上
    app.popupTimer = setTimeout(() => {
      DOM.taskPopup.classList.add('hidden');
    }, 200);
  });

  DOM.taskPopup.addEventListener('mouseenter', () => {
    clearTimeout(app.popupTimer);
  });

  DOM.taskPopup.addEventListener('mouseleave', () => {
    DOM.taskPopup.classList.add('hidden');
  });

  DOM.btnQuickAdd.addEventListener('click', () => {
    DOM.taskPopup.classList.add('hidden');
    openAddModal();
  });

  DOM.btnViewAll.addEventListener('click', () => {
    DOM.taskPopup.classList.add('hidden');
    openTaskModal();
  });
}

// buildPopupHtml removed — using DOM popup rendering

// ═══════════════════════════════════════════════════════════════
// 5. 任务管理弹窗
// ═══════════════════════════════════════════════════════════════

function openTaskModal() {
  DOM.taskModal.classList.remove('hidden');
  loadTaskList(app.currentFilter);
}

async function loadTaskList(filter) {
  app.currentFilter = filter;
  try {
    const tasks = await invoke('get_tasks', { filter });
    DOM.taskList.innerHTML = '';

    // 更新 tab active 状态
    $$('.tab').forEach((t) => {
      t.classList.toggle('active', t.dataset.filter === filter);
    });

    if (tasks.length === 0) {
      DOM.taskList.innerHTML = '<div style="text-align:center;padding:24px;color:var(--text-light)">暂无任务</div>';
      return;
    }

    tasks.forEach((t) => {
      const item = document.createElement('div');
      item.className = 'task-item';

      const checkbox = document.createElement('input');
      checkbox.type = 'checkbox';
      checkbox.checked = t.is_completed;
      checkbox.disabled = t.is_completed;
      checkbox.addEventListener('change', () => completeTask(t));

      const info = document.createElement('div');
      info.className = 'task-info';

      const title = document.createElement('div');
      title.className = `task-title${t.is_completed ? ' done' : ''}`;
      title.textContent = t.title;

      const time = document.createElement('div');
      const overdue = !t.is_completed && isOverdue(t.due_time);
      time.className = `task-time${overdue ? ' urgent' : ''}`;
      time.textContent = formatDateTime(t.due_time);

      info.appendChild(title);
      info.appendChild(time);

      const del = document.createElement('button');
      del.className = 'task-del';
      del.textContent = '×';
      del.addEventListener('click', () => deleteTask(t.id));

      item.appendChild(checkbox);
      item.appendChild(info);
      if (!t.is_completed) item.appendChild(del);

      DOM.taskList.appendChild(item);
    });
  } catch (e) {
    console.error('loadTaskList failed:', e);
  }
}

async function completeTask(task) {
  try {
    await invoke('complete_task', {
      id: task.id,
      title: task.title,
      voiceEnabled: settings.voice_enabled === 'true',
    });
    showBubble('好棒！任务完成啦 🎉', 2500);
    loadTaskList(app.currentFilter);
  } catch (e) {
    console.error('completeTask failed:', e);
  }
}

async function deleteTask(id) {
  try {
    await invoke('delete_task', { id });
    loadTaskList(app.currentFilter);
  } catch (e) {
    console.error('deleteTask failed:', e);
  }
}

function bindTaskModalEvents() {
  // Tab 切换
  $$('.tab').forEach((tab) => {
    tab.addEventListener('click', () => {
      loadTaskList(tab.dataset.filter);
    });
  });

  // 添加任务按钮
  DOM.btnAddTask.addEventListener('click', () => {
    openAddModal();
  });
}

// ═══════════════════════════════════════════════════════════════
// 6. 添加任务弹窗
// ═══════════════════════════════════════════════════════════════

function openAddModal() {
  // 预设截止时间为当前时间 +1 小时（取整）
  const now = new Date();
  now.setHours(now.getHours() + 1);
  now.setMinutes(0, 0, 0);
  DOM.inpDue.value = toLocalDatetime(now);

  DOM.inpTitle.value = '';
  DOM.inpDesc.value = '';
  DOM.inpRemind.value = '15';

  DOM.addModal.classList.remove('hidden');
  setTimeout(() => DOM.inpTitle.focus(), 100);
}

function toLocalDatetime(d) {
  const pad = (n) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

async function saveTask() {
  const title = DOM.inpTitle.value.trim();
  const dueRaw = DOM.inpDue.value;

  if (!title) {
    DOM.inpTitle.focus();
    showBubble('请输入任务名称', 2000);
    return;
  }
  if (!dueRaw) {
    DOM.inpDue.focus();
    showBubble('请选择截止时间', 2000);
    return;
  }

  // datetime-local 格式 "2026-06-10T14:00" → "2026-06-10 14:00:00"
  const dueTime = dueRaw.replace('T', ' ') + ':00';
  const description = DOM.inpDesc.value.trim();
  const remindMinutes = parseInt(DOM.inpRemind.value, 10);

  try {
    await invoke('add_task', {
      title,
      description,
      dueTime,
      remindMinutes,
      voiceEnabled: settings.voice_enabled === 'true',
    });
    DOM.addModal.classList.add('hidden');
    showBubble(`收到新任务: ${title}`, 3000);
    // 刷新任务列表（如果打开的话）
    if (!DOM.taskModal.classList.contains('hidden')) {
      loadTaskList(app.currentFilter);
    }
  } catch (e) {
    console.error('saveTask failed:', e);
    showBubble('保存失败', 2000);
  }
}

function bindAddModalEvents() {
  DOM.btnSave.addEventListener('click', saveTask);

  // Enter 键保存
  DOM.addModal.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && e.target.tagName !== 'TEXTAREA') {
      e.preventDefault();
      saveTask();
    }
  });
}

// ═══════════════════════════════════════════════════════════════
// 7. 设置弹窗
// ═══════════════════════════════════════════════════════════════

function bindSettingsModalEvents() {
  // Tab 切换
  $$('.stab').forEach((stab) => {
    stab.addEventListener('click', () => {
      const panelId = stab.dataset.panel;
      // 切换 tab active
      $$('.stab').forEach((s) => s.classList.remove('active'));
      stab.classList.add('active');
      // 切换 panel 可见性
      $$('.spanel').forEach((p) => p.classList.add('hidden'));
      $(`#${panelId}`).classList.remove('hidden');
    });
  });
}

// ═══════════════════════════════════════════════════════════════
// 8. 弹窗通用关闭逻辑
// ═══════════════════════════════════════════════════════════════

function bindModalClose() {
  // data-close 按钮
  document.addEventListener('click', (e) => {
    const closeTarget = e.target.dataset.close;
    if (closeTarget) {
      $(`#${closeTarget}`).classList.add('hidden');
    }
  });

  // 点击遮罩层关闭
  document.addEventListener('click', (e) => {
    if (e.target.classList.contains('overlay')) {
      e.target.classList.add('hidden');
    }
  });
}

// ═══════════════════════════════════════════════════════════════
// 9. 右键菜单
// ═══════════════════════════════════════════════════════════════

function bindContextMenu() {
  // 右键显示菜单
  DOM.petArea.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    DOM.ctxMenu.style.left = `${e.clientX}px`;
    DOM.ctxMenu.style.top = `${e.clientY}px`;
    DOM.ctxMenu.classList.remove('hidden');
  });

  // 点击其他地方关闭菜单
  document.addEventListener('click', () => {
    DOM.ctxMenu.classList.add('hidden');
  });

  // 菜单项处理
  $$('.ctx-item').forEach((item) => {
    item.addEventListener('click', (e) => {
      e.stopPropagation();
      DOM.ctxMenu.classList.add('hidden');
      handleCtxAction(item.dataset.act);
    });
  });
}

async function handleCtxAction(act) {
  switch (act) {
    case 'add':
      openAddModal();
      break;
    case 'list':
      openTaskModal();
      break;
    case 'rest':
      toggleRestMode();
      break;
    case 'mute':
      toggleMute();
      break;
    case 'settings':
      DOM.settingsModal.classList.remove('hidden');
      await loadSettings();
      break;
    case 'exit':
      try {
        await getCurrentWindow().close();
      } catch (e) {
        console.error('exit failed:', e);
      }
      break;
  }
}

async function toggleRestMode() {
  app.restMode = !app.restMode;
  if (app.restMode) {
    try {
      await invoke('set_pet_state', {
        stateName: 'SLEEPING',
        sadnessLevel: 0,
        message: '休息中...',
      });
      showBubble('进入休息模式 💤', 2000);
    } catch (e) {
      console.error('toggleRestMode failed:', e);
    }
  } else {
    try {
      await invoke('set_pet_state', {
        stateName: 'IDLE',
        sadnessLevel: 0,
        message: '我回来了!',
      });
      showBubble('我回来了!', 2000);
    } catch (e) {
      console.error('toggleRestMode failed:', e);
    }
  }
}

function toggleMute() {
  app.muted = !app.muted;
  showBubble(app.muted ? '已静音 🔇' : '已取消静音 🔔', 1500);
}

// ═══════════════════════════════════════════════════════════════
// 10. 猫咪交互（点击、长按摸头）
// ═══════════════════════════════════════════════════════════════

const IDLE_BUBBLES = [
  '喵~',
  '今天也要加油哦!',
  '休息一下吧~',
  '有什么需要帮忙的吗?',
  '摸摸头 >w<',
  '记得喝水哦!',
  '喵呜~',
];

function bindCatInteractions() {
  let pressTimer = null;
  let isLongPress = false;

  DOM.cat.addEventListener('mousedown', (e) => {
    if (e.button !== 0) return; // 仅左键
    isLongPress = false;

    pressTimer = setTimeout(async () => {
      isLongPress = true;
      // 长按 → 摸头
      try {
        await invoke('pet_pet');
        showBubble('被摸头了~ 好开心! 💕', 2500);
      } catch (e) {
        console.error('pet_pet failed:', e);
      }
    }, 1000);
  });

  DOM.cat.addEventListener('mouseup', () => {
    clearTimeout(pressTimer);
  });

  DOM.cat.addEventListener('mouseleave', () => {
    clearTimeout(pressTimer);
  });

  // 单击 → 随机气泡
  DOM.cat.addEventListener('click', (e) => {
    if (isLongPress) return;
    const text = IDLE_BUBBLES[Math.floor(Math.random() * IDLE_BUBBLES.length)];
    showBubble(text, 2500);
  });

  // 双击 → 打开任务管理
  DOM.cat.addEventListener('dblclick', () => {
    openTaskModal();
  });
}

// ═══════════════════════════════════════════════════════════════
// 11. 拖拽 & 贴边吸附
// ═══════════════════════════════════════════════════════════════

function bindDrag() {
  DOM.cat.addEventListener('mousedown', async (e) => {
    if (e.button !== 0) return;
    try {
      await getCurrentWindow().startDragging();
    } catch (e) {
      console.warn('startDragging not available:', e);
    }
  });
}

// ═══════════════════════════════════════════════════════════════
// 12. 整点提醒
// ═══════════════════════════════════════════════════════════════

function startHourlyCheck() {
  setInterval(async () => {
    const now = new Date();
    const hour = now.getHours();
    const minute = now.getMinutes();

    if (minute !== 0) return;
    if (hour === app.lastCheckedHour) return;
    if (settings.hourly_enabled !== 'true') return;

    const startH = parseInt(settings.hourly_start_hour, 10);
    const endH = parseInt(settings.hourly_end_hour, 10);
    if (hour < startH || hour > endH) return;

    app.lastCheckedHour = hour;

    try {
      if (!app.muted) {
        await invoke('hour_reached', {
          hour,
          voiceEnabled: settings.voice_enabled === 'true',
        });
      }
      // 时段问候语
      const greetings = {
        6: '早安! 新的一天开始啦 🌅',
        7: '早安! 新的一天开始啦 🌅',
        8: '早安! 新的一天开始啦 🌅',
        9: '上午好! 继续加油 💪',
        10: '上午好! 继续加油 💪',
        11: '快中午了~',
        12: '中午啦! 该吃饭了 🍚',
        13: '午休一下吧 😴',
        14: '下午好! 继续加油 ☕',
        15: '下午好! 继续加油 ☕',
        16: '快下班了~',
        17: '傍晚好 🌆',
        18: '晚上好~',
        19: '晚上好~',
        20: '晚上好~ 放松一下吧',
        21: '不早了~',
        22: '很晚了，早点休息吧 🌙',
        23: '深夜了，注意休息 🌙',
      };
      const msg = greetings[hour] || `${hour}点了`;
      showBubble(msg, 4000);
    } catch (e) {
      console.error('hour_reached failed:', e);
    }
  }, 60000); // 每分钟检查一次
}

// ═══════════════════════════════════════════════════════════════
// 13. 空闲/睡觉检测
// ═══════════════════════════════════════════════════════════════

function startIdleDetection() {
  const IDLE_TIMEOUT = 5 * 60 * 1000; // 5 分钟

  function resetIdleTimer() {
    clearTimeout(app.idleTimer);

    // 如果正在睡觉，唤醒
    if (app.isSleeping && !app.restMode) {
      invoke('set_pet_state', {
        stateName: 'IDLE',
        sadnessLevel: 0,
        message: '你回来啦!',
      }).then(() => {
        showBubble('你回来啦! 🎉', 2000);
      });
    }

    app.idleTimer = setTimeout(async () => {
      // 空闲超时 → 进入睡眠
      if (!app.restMode) {
        try {
          await invoke('set_pet_state', {
            stateName: 'SLEEPING',
            sadnessLevel: 0,
            message: '好困...',
          });
        } catch (e) {
          console.error('idle sleep failed:', e);
        }
      }
    }, IDLE_TIMEOUT);
  }

  document.addEventListener('mousemove', resetIdleTimer);
  document.addEventListener('keydown', resetIdleTimer);
  document.addEventListener('click', resetIdleTimer);
  resetIdleTimer();
}

// ═══════════════════════════════════════════════════════════════
// 14. 初始化
// ═══════════════════════════════════════════════════════════════

document.addEventListener('DOMContentLoaded', async () => {
  cacheDom();

  // 绑定所有事件
  bindModalClose();
  bindPopupEvents();
  bindTaskModalEvents();
  bindAddModalEvents();
  bindSettingsModalEvents();
  bindContextMenu();
  bindCatInteractions();
  bindDrag();
  bindSettingsControls();

  // 加载数据
  await loadSettings();

  // 启动定时任务
  setInterval(pollPetState, 2000);
  pollPetState();
  startHourlyCheck();
  startIdleDetection();
});
