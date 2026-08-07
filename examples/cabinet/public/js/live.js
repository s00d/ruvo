/* Live WebSocket island. */
(function () {
  const { createApp } = window.Vue || {};
  const el = document.getElementById('vue-live');
  if (!createApp || !el) return;

  createApp({
    data() {
      return {
        log: [],
        msg: '',
        status: 'connecting',
        ws: null,
      };
    },
    mounted() {
      const proto = location.protocol === 'https:' ? 'wss' : 'ws';
      const ws = new WebSocket(`${proto}://${location.host}/cabinet/ws`);
      this.ws = ws;
      ws.onopen = () => {
        this.status = 'live';
      };
      ws.onclose = () => {
        this.status = 'closed';
      };
      ws.onerror = () => {
        this.status = 'error';
      };
      ws.onmessage = (e) => {
        this.log.push({ id: Date.now() + Math.random(), text: String(e.data) });
        this.$nextTick(() => {
          const box = this.$refs.logBox;
          if (box) box.scrollTop = box.scrollHeight;
        });
      };
    },
    beforeUnmount() {
      if (this.ws) this.ws.close();
    },
    methods: {
      send(e) {
        e.preventDefault();
        const text = this.msg.trim();
        if (!text || !this.ws || this.ws.readyState !== WebSocket.OPEN) return;
        this.ws.send(text);
        this.msg = '';
      },
    },
  }).mount(el);
})();
