// Donkr's entire frontend. Pages work without it; this just makes carrots and chat feel instant.

// 🥕 Send a carrot without leaving the profile
document.querySelectorAll('button.carrot').forEach((btn) => {
  btn.addEventListener('click', async () => {
    if (btn.classList.contains('sent')) return;
    const res = await fetch(`/d/${btn.dataset.id}/carrot`, { method: 'POST' });
    if (!res.ok) return;
    const { carrots } = await res.json();
    btn.classList.add('sent', 'pop');
    btn.textContent = '🥕 Carrot sent';
    const count = document.querySelector(`[data-carrots="${btn.dataset.id}"]`);
    if (count) count.textContent = carrots;
    const r = btn.getBoundingClientRect();
    const fly = Object.assign(document.createElement('span'), { className: 'flying', textContent: '🥕' });
    fly.style.left = `${r.left + r.width / 2 - 17}px`;
    fly.style.top = `${r.top - 10}px`;
    document.body.append(fly);
    setTimeout(() => fly.remove(), 1000);
  });
});

// 💬 Chat: post as JSON, append both bubbles, keep the thread scrolled to the bottom
const thread = document.getElementById('thread');
const scrollDown = () => thread && thread.lastElementChild?.scrollIntoView({ block: 'end' });
scrollDown();
const bubble = (m) => {
  const div = document.createElement('div');
  div.className = `bubble${m.from_me ? ' me' : ''}`;
  div.textContent = m.body;
  const t = Object.assign(document.createElement('span'), { className: 'time', textContent: m.sent_at });
  div.append(t);
  return div;
};
document.querySelectorAll('form.composer').forEach((form) => {
  form.addEventListener('submit', async (e) => {
    e.preventDefault();
    const input = form.querySelector('input');
    const body = input.value.trim();
    if (!body) return;
    input.value = '';
    const res = await fetch(`/api/chat/${form.dataset.id}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ body }),
    });
    if (!res.ok) return;
    const { mine, reply } = await res.json();
    thread.querySelector('.empty')?.remove();
    thread.append(bubble(mine));
    scrollDown();
    setTimeout(() => { thread.append(bubble(reply)); scrollDown(); }, 900); // donkeys type slowly (hooves)
  });
});

// 📣 Recruits: fake invites, real satisfaction
document.querySelectorAll('button.invite').forEach((btn) => {
  btn.addEventListener('click', () => {
    btn.textContent = 'Invited ✓';
    btn.classList.add('done', 'pop');
  });
});
