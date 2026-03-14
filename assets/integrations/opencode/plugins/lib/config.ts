import { readFileSync } from "node:fs";
import { join } from "node:path";

type OpenCodeConfig = {
  mcp?: {
    maenifold?: {
      environment?: Record<string, unknown>;
    };
  };
};

function getOpenCodeConfigPath(): string | undefined {
  const home = process.env.HOME?.trim();
  if (!home) {
    return undefined;
  }

  return join(home, ".config", "opencode", "opencode.json");
}

function stripJsonComments(text: string): string {
  let result = "";
  let inString = false;
  let isEscaped = false;
  let inLineComment = false;
  let inBlockComment = false;

  for (let index = 0; index < text.length; index++) {
    const current = text[index];
    const next = text[index + 1];

    if (inLineComment) {
      if (current === "\n" || current === "\r") {
        inLineComment = false;
        result += current;
      }
      continue;
    }

    if (inBlockComment) {
      if (current === "*" && next === "/") {
        inBlockComment = false;
        index++;
      }
      continue;
    }

    if (inString) {
      result += current;

      if (isEscaped) {
        isEscaped = false;
        continue;
      }

      if (current === "\\") {
        isEscaped = true;
        continue;
      }

      if (current === '"') {
        inString = false;
      }

      continue;
    }

    if (current === '"') {
      inString = true;
      result += current;
      continue;
    }

    if (current === "/" && next === "/") {
      inLineComment = true;
      index++;
      continue;
    }

    if (current === "/" && next === "*") {
      inBlockComment = true;
      index++;
      continue;
    }

    result += current;
  }

  return result;
}

function stripTrailingCommas(text: string): string {
  let result = "";
  let inString = false;
  let isEscaped = false;

  for (let index = 0; index < text.length; index++) {
    const current = text[index];

    if (inString) {
      result += current;

      if (isEscaped) {
        isEscaped = false;
        continue;
      }

      if (current === "\\") {
        isEscaped = true;
        continue;
      }

      if (current === '"') {
        inString = false;
      }

      continue;
    }

    if (current === '"') {
      inString = true;
      result += current;
      continue;
    }

    if (current === ",") {
      let lookahead = index + 1;
      while (lookahead < text.length && /\s/.test(text[lookahead] ?? "")) {
        lookahead++;
      }

      const nextToken = text[lookahead];
      if (nextToken === "}" || nextToken === "]") {
        continue;
      }
    }

    result += current;
  }

  return result;
}

function parseOpenCodeConfig(raw: string): OpenCodeConfig | undefined {
  try {
    return JSON.parse(
      stripTrailingCommas(stripJsonComments(raw)),
    ) as OpenCodeConfig;
  } catch {
    return undefined;
  }
}

function readOpenCodeConfigEnvironment(): Record<string, unknown> | undefined {
  const configPath = getOpenCodeConfigPath();
  if (!configPath) {
    return undefined;
  }

  try {
    const config = parseOpenCodeConfig(readFileSync(configPath, "utf8"));
    const environment = config?.mcp?.maenifold?.environment;

    if (
      !environment ||
      typeof environment !== "object" ||
      Array.isArray(environment)
    ) {
      return undefined;
    }

    return environment;
  } catch {
    return undefined;
  }
}

function coerceEnvValue(value: unknown): string | undefined {
  if (typeof value === "string") {
    return value;
  }

  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }

  return undefined;
}

export function getMaenifoldPluginEnv(name: string): string | undefined {
  if (Object.prototype.hasOwnProperty.call(process.env, name)) {
    return process.env[name];
  }

  return coerceEnvValue(readOpenCodeConfigEnvironment()?.[name]);
}

export function getMaenifoldPluginFlag(name: string): boolean | undefined {
  const value = getMaenifoldPluginEnv(name)?.trim().toLowerCase();

  switch (value) {
    case "1":
    case "true":
    case "yes":
    case "on":
      return true;
    case "0":
    case "false":
    case "no":
    case "off":
      return false;
    default:
      return undefined;
  }
}
