import { listen } from '@tauri-apps/api/event';
import { window } from '@tauri-apps/api';

const popup = document.getElementById('child-popup');
const content = document.getElementById('child-content');

listen('popup:show', event => {
  const payload = event.payload;
  if (!payload) return;
  content.innerHTML = payload.html || '';
  popup.classList.remove('hidden');
});

listen('ctx:show', event => {
  const payload = event.payload;
  if (!payload) return;
  // sample static menu
  content.innerHTML = `
    <div class="ctx-item">📋 添加任务</div>
    <div class="ctx-item">📅 查看全部任务</div>
    <div class="ctx-sep"></div>
    <div class="ctx-item">⚙️ 设置</div>
  `;
  popup.classList.remove('hidden');
});

// Hide on click outside
window.appWindow.once('tauri://close', () => {});
window.addEventListener('click', () => {
  popup.classList.add('hidden');
});
