import { defineStore } from "pinia";
import { ref } from "vue";
import { postAction, type ActionResponse } from "../api";

const REDIS_DB_KEY = "sova.devtools.console.redisDb";

function loadRedisDb(): number {
  try {
    const n = Number(sessionStorage.getItem(REDIS_DB_KEY));
    return Number.isFinite(n) && n >= 0 && n <= 15 ? n : 0;
  } catch {
    return 0;
  }
}

export const useConsoleStore = defineStore("console", () => {
  const loading = ref(false);
  const error = ref<string | null>(null);
  const last = ref<ActionResponse | null>(null);

  const redisDb = ref(loadRedisDb());
  const redisOp = ref<
    "get" | "set" | "del" | "publish" | "ttl" | "scan" | "type"
  >("get");
  const redisKey = ref("");
  const redisValue = ref("");
  const redisChannel = ref("");
  const redisPattern = ref("*");
  const redisCursor = ref(0);
  const redisTtl = ref(0);
  const redisMessages = ref<{ channel: string; payload: string }[]>([]);
  const redisListening = ref(false);

  let redisEs: EventSource | null = null;

  function setRedisDb(n: number) {
    redisDb.value = Math.min(15, Math.max(0, n));
    try {
      sessionStorage.setItem(REDIS_DB_KEY, String(redisDb.value));
    } catch {
      /* ignore */
    }
  }

  function prefillRedisKey(key: string) {
    redisKey.value = key;
    redisOp.value = "get";
  }

  async function runRedis(api: string) {
    loading.value = true;
    error.value = null;
    const payload: Record<string, unknown> = {
      op: redisOp.value,
      db: redisDb.value,
    };
    if (redisKey.value) payload.key = redisKey.value;
    if (redisValue.value) payload.value = redisValue.value;
    if (redisChannel.value) payload.channel = redisChannel.value;
    if (redisOp.value === "scan") {
      payload.pattern = redisPattern.value || "*";
      payload.cursor = redisCursor.value;
    }
    if (redisOp.value === "set" && redisTtl.value > 0) {
      payload.ttl_secs = redisTtl.value;
    }
    try {
      last.value = await postAction(api, "redis", payload);
      if (!last.value.ok) {
        error.value = last.value.error ?? "redis failed";
      } else if (
        redisOp.value === "scan" &&
        last.value.result &&
        typeof last.value.result === "object"
      ) {
        const r = last.value.result as { cursor?: number };
        if (typeof r.cursor === "number") redisCursor.value = r.cursor;
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      loading.value = false;
    }
  }

  function stopRedisListen() {
    redisEs?.close();
    redisEs = null;
    redisListening.value = false;
  }

  function toggleRedisListen(api: string) {
    if (redisListening.value) {
      stopRedisListen();
      return;
    }
    if (!redisChannel.value.trim()) {
      error.value = "channel required";
      return;
    }
    stopRedisListen();
    redisMessages.value = [];
    const url = `${api}/stream/redis?channel=${encodeURIComponent(redisChannel.value)}`;
    redisEs = new EventSource(url);
    redisListening.value = true;
    redisEs.addEventListener("message", (ev) => {
      try {
        const data = JSON.parse((ev as MessageEvent).data) as {
          channel?: string;
          payload?: string;
        };
        redisMessages.value = [
          {
            channel: data.channel ?? redisChannel.value,
            payload: data.payload ?? "",
          },
          ...redisMessages.value,
        ].slice(0, 50);
      } catch {
        /* ignore */
      }
    });
    redisEs.onerror = () => stopRedisListen();
  }

  return {
    loading,
    error,
    last,
    redisDb,
    redisOp,
    redisKey,
    redisValue,
    redisChannel,
    redisPattern,
    redisCursor,
    redisTtl,
    redisMessages,
    redisListening,
    prefillRedisKey,
    runRedis,
    setRedisDb,
    toggleRedisListen,
    stopRedisListen,
  };
});
