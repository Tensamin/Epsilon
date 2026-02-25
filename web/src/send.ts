import { z } from "zod";
import { log } from "@tensamin/shared/log";
import { RESPONSE_TIMEOUT } from "./values";

export type TypedMessage<TData = Record<string, unknown>> = {
  id: string;
  type: string;
  data: TData;
};

export type Message = TypedMessage;

type SchemaMap = Record<string, { request: z.ZodType; response: z.ZodType }>;

type PendingRequest = {
  type: string;
  resolve: (value: TypedMessage) => void;
  reject: (reason: unknown) => void;
  timeoutId: ReturnType<typeof setTimeout>;
};

type PendingMap = Map<string, PendingRequest>;

type SendOptions = {
  id?: string;
  noResponse?: boolean;
};

type SendFn<T extends SchemaMap> = {
  <K extends keyof T & string>(
    socket: WebSocket,
    type: K,
    data: z.input<T[K]["request"]>,
    options: SendOptions & { noResponse: true },
  ): Promise<void>;

  <K extends keyof T & string>(
    socket: WebSocket,
    type: K,
    data: z.input<T[K]["request"]>,
    options?: SendOptions & { noResponse?: false },
  ): Promise<TypedMessage<z.output<T[K]["response"]>>>;
};

export function createPendingRequests<T extends SchemaMap>(schemas: T) {
  const map: PendingMap = new Map();

  function handleMessage(event: MessageEvent) {
    const parsed: TypedMessage = JSON.parse(String(event.data));
    if (parsed.type !== "pong") {
      log(2, "Socket", "blue", parsed.type, parsed.data);
    }
    const pending = map.get(parsed.id);
    if (!pending) return;

    clearTimeout(pending.timeoutId);
    map.delete(parsed.id);

    if (parsed.type.startsWith("error")) {
      pending.reject(parsed);
      return;
    }

    const schema = schemas[pending.type];
    if (schema) {
      const result = schema.response.safeParse(parsed.data);
      if (result.success) {
        pending.resolve({
          ...parsed,
          data: result.data as Record<string, unknown>,
        });
      } else {
        log(
          0,
          "Socket",
          "red",
          `Response validation failed for "${parsed.type}"`,
          result.error,
          parsed.data,
        );
        pending.reject(
          new Error(
            `Response validation failed for "${parsed.type}": ${result.error.message}`,
          ),
        );
      }
    } else {
      pending.resolve(parsed);
    }
  }

  function cleanup() {
    const error = new Error("Socket closed with pending requests");
    for (const [, { reject, timeoutId }] of map) {
      clearTimeout(timeoutId);
      reject(error);
    }
    map.clear();
  }

  const send: SendFn<T> = ((
    socket: WebSocket,
    type: string,
    data?: Record<string, unknown>,
    options?: SendOptions,
  ): Promise<TypedMessage> | Promise<void> => {
    const schema = schemas[type];

    // Validate outgoing data against request schema
    if (schema) {
      const result = schema.request.safeParse(data ?? {});
      if (!result.success) {
        log(
          0,
          "Socket",
          "red",
          `Request validation failed for "${type}"`,
          result.error,
        );
        return Promise.reject(
          new Error(
            `Request validation failed for "${type}": ${result.error.message}`,
          ),
        ) as Promise<TypedMessage>;
      }
      data = result.data as Record<string, unknown>;
    }

    if (type !== "ping") {
      log(2, "Socket", "purple", type, data);
    }

    const id = options?.id ?? crypto.randomUUID();
    const message = JSON.stringify({ id, type, data: data ?? {} });

    if (options?.noResponse) {
      return new Promise<void>((resolve, reject) => {
        try {
          socket.send(message);
          resolve();
        } catch (error) {
          reject(error);
        }
      });
    }

    return new Promise<TypedMessage>((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        map.delete(id);
        reject(
          new Error(`Request "${type}" timed out after ${RESPONSE_TIMEOUT}ms`),
        );
      }, RESPONSE_TIMEOUT);

      map.set(id, { type, resolve, reject, timeoutId });

      try {
        socket.send(message);
      } catch (error) {
        clearTimeout(timeoutId);
        map.delete(id);
        reject(error);
      }
    });
  }) as unknown as SendFn<T>;

  return { handleMessage, cleanup, send };
}
