import { log } from "@tensamin/shared/log";
import {
  createContext,
  createEffect,
  createSignal,
  onCleanup,
  Show,
  untrack,
  useContext,
  type ParentProps,
} from "solid-js";
import { z } from "zod";

import { createPendingRequests, type TypedMessage } from "./send";
import { RETRY_COUNT, RETRY_INTERVAL, PING_INTERVAL } from "./values";
import {
  socket as schemas,
  type Socket as Schemas,
} from "@tensamin/shared/data";
import { useStorage } from "@tensamin/core-storage/context";
import { useCrypto } from "@tensamin/core-crypto/context";
import Loading from "@tensamin/ui/screens/loading";
import ErrorScreen from "@tensamin/ui/screens/error";
import { useNavigate } from "@solidjs/router";

type SchemaMap = Record<string, { request: z.ZodType; response: z.ZodType }>;

type BoundSendFn<T extends SchemaMap> = {
  <K extends keyof T & string>(
    type: K,
    data: z.input<T[K]["request"]>,
    options: { id?: string; noResponse: true },
  ): Promise<void>;

  <K extends keyof T & string>(
    type: K,
    data: z.input<T[K]["request"]>,
    options?: { id?: string; noResponse?: false },
  ): Promise<TypedMessage<z.output<T[K]["response"]>>>;
};

type OmikronData = {
  id: number;
  public_key: string;
  ip_address: string;
};

type ContextType = {
  send: BoundSendFn<Schemas>;
  readyState: () => number;
  ownPing: () => number;
  iotaPing: () => number;
};

const socketContext = createContext<ContextType>();

export default function Provider(props: ParentProps) {
  const pending = createPendingRequests(schemas);

  const [omikron, setOmikron] = createSignal<OmikronData | null>(null);
  const [url, setUrl] = createSignal<string | null>(null);
  const [readyState, setReadyState] = createSignal<number>(WebSocket.CLOSED);
  const [identified, setIdentified] = createSignal<boolean>(false);

  const [ownPing, setOwnPing] = createSignal<number>(0);
  const [iotaPing, setIotaPing] = createSignal<number>(0);

  const [error, setError] = createSignal<string>("");
  const [errorDescription, setErrorDescription] = createSignal<string>("");

  const { load } = useStorage();
  const { get_shared_secret, decrypt } = useCrypto();

  const navigate = useNavigate();

  let ws: WebSocket | null = null;

  // Load Omikron
  createEffect(() => {
    if (omikron()) return;

    const controller = new AbortController();

    (async () => {
      try {
        const userId = await load("user_id");

        // Redirect to login
        if (userId === 0) {
          navigate("/login");
          return;
        }

        const res = await fetch(
          "https://omega.tensamin.net/api/get/omikron/" + String(userId),
          { signal: controller.signal },
        );
        const data = await res.json();
        setOmikron(data);
        setUrl(data.ip_address + "/ws/client/");
      } catch (e) {
        if (controller.signal.aborted) return;

        setError("Failed to load Omikron data");
        setErrorDescription(
          "An error occurred while fetching the Omikron server data. Please try again later.",
        );
        log(0, "Socket", "red", "Failed to fetch Omikron data", e);
      }
    })();

    onCleanup(() => controller.abort());
  });

  createEffect(() => {
    if (identified()) {
      const interval = setInterval(async () => {
        const originalNow = Date.now();

        const data = await send("ping", {
          last_ping: originalNow,
        });

        const travelTime = Date.now() - originalNow;

        setOwnPing(travelTime);
        setIotaPing(data.data.ping_iota);
      }, PING_INTERVAL);

      onCleanup(() => clearInterval(interval));
    }
  });

  // Create WebSocket with reconnection
  createEffect(() => {
    const currentUrl = url();
    if (!currentUrl) return;

    let attempts = 0;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
    let disposed = false;

    async function identify() {
      const userId = await load("user_id");
      const privateKey = await load("private_key");
      const omikronData = untrack(() => omikron());

      send("identification", { user_id: userId })
        .then(async (data) => {
          const ownUserData = await send("get_user_data", {
            user_id: userId,
          });

          if (!omikronData?.public_key) {
            setError("Omikron data missing");
            setErrorDescription(
              "Omikron server data is missing. Please try again later.",
            );
            log(0, "Socket", "red", "Omikron public key missing");
            return;
          }

          try {
            const sharedSecret = await get_shared_secret(
              privateKey,
              ownUserData.data.public_key,
              omikronData.public_key,
            );

            const solvedChallenge = await decrypt(
              sharedSecret,
              data.data.challenge,
            );

            send("challenge_response", {
              challenge: btoa(solvedChallenge),
            })
              .then(() => {
                log(1, "Socket", "green", "Identification successful");
                setIdentified(true);
                setError("");
                setErrorDescription("");
              })
              .catch((error) => {
                log(0, "Socket", "red", "Challenge failed", error);
                setError("Challenge Failed");
                setErrorDescription(
                  "Failed to respond to the server's challenge. Please try again later.",
                );
              });
          } catch (err) {
            setError("Validation Failed");
            setErrorDescription(
              "Failed to validate the server's identity. Please try again later.",
            );
            log(0, "Socket", "red", "Server identity validation failed", err);
          }
        })
        .catch((e) => {
          log(0, "Socket", "red", "Identification failed", e);
          setError("Identification Failed");
          setErrorDescription(
            "Failed to identify with the server. Please try again later.",
          );
        });
    }

    function connect() {
      if (disposed) return;

      const socket = new WebSocket(currentUrl!);

      socket.onopen = () => {
        log(1, "Socket", "green", "Connected");
        attempts = 0;
        setReadyState(WebSocket.OPEN);
        identify();
      };

      socket.onclose = () => {
        log(0, "Socket", "red", "Disconnected");
        setReadyState(WebSocket.CLOSED);
        pending.cleanup();

        if (disposed) return;

        if (attempts < RETRY_COUNT) {
          attempts++;
          reconnectTimer = setTimeout(connect, RETRY_INTERVAL);
        } else {
          setError("Connection Failed");
          setErrorDescription(
            "Unable to connect to the server after multiple attempts. Please check your internet connection or try again later.",
          );
          log(0, "Socket", "red", "Reconnection attempts exhausted");
        }
      };

      socket.onerror = (e) => {
        setError("Connection Error");
        setErrorDescription(
          "An error occurred with the WebSocket connection. Please check your internet connection or try again later.",
        );
        log(0, "Socket", "red", "WebSocket error", e);
      };

      socket.onmessage = (event) => {
        pending.handleMessage(event);
      };

      ws = socket;
      setReadyState(WebSocket.CONNECTING);
    }

    connect();

    onCleanup(() => {
      disposed = true;
      if (reconnectTimer) clearTimeout(reconnectTimer);
      if (ws) {
        ws.close();
        ws = null;
      }
      setReadyState(WebSocket.CLOSED);
    });
  });

  // Create Send Function
  const send: BoundSendFn<Schemas> = ((
    type: string,
    data?: Record<string, unknown>,
    options?: { id?: string; noResponse?: boolean },
  ): Promise<TypedMessage> | Promise<void> => {
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      return Promise.reject(
        new Error("Socket is not connected"),
      ) as Promise<TypedMessage>;
    }

    if (options?.noResponse) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      return pending.send(ws, type as keyof Schemas & string, data as any, {
        ...options,
        noResponse: true,
      });
    }
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    return pending.send(ws, type as keyof Schemas & string, data as any, {
      ...options,
      noResponse: false,
    });
  }) as unknown as BoundSendFn<Schemas>;

  const progress = () => {
    if (!omikron()) return 40;
    if (!url()) return 70;
    if (!identified()) return 90;
    return 100;
  };

  return (
    <Show
      when={error() === "" && errorDescription() === ""}
      fallback={
        <ErrorScreen error={error()} description={errorDescription()} />
      }
    >
      <Show
        when={omikron() && url() && identified()}
        fallback={<Loading progress={progress()} />}
      >
        <socketContext.Provider value={{ send, readyState, ownPing, iotaPing }}>
          {props.children}
        </socketContext.Provider>
      </Show>
    </Show>
  );
}

export function useSocket(): ContextType {
  const context = useContext(socketContext);
  if (!context)
    throw new Error("useSocket must be used within a SocketProvider");
  return context;
}
