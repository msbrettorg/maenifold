using System.Text.Json;

namespace Maenifold.Utils
{
    public static class PayloadReader
    {
        public static string GetString(JsonElement payload, string property, bool required = true, string? defaultValue = null)
        {
            if (!payload.TryGetProperty(property, out JsonElement element))
            {
                if (required)
                {
                    throw new ArgumentException($"Required property '{property}' is missing from payload.");
                }
                return defaultValue ?? string.Empty;
            }

            if (element.ValueKind != JsonValueKind.String)
            {
                throw new ArgumentException($"Property '{property}' must be a string, but found {element.ValueKind}.");
            }

            string? value = element.GetString();
            if (required && string.IsNullOrWhiteSpace(value))
            {
                throw new ArgumentException($"Required property '{property}' cannot be empty or whitespace.");
            }

            return value ?? string.Empty;
        }

        public static bool GetBool(JsonElement payload, string property, bool defaultValue = false)
        {
            if (!payload.TryGetProperty(property, out JsonElement element))
            {
                return defaultValue;
            }

            if (element.ValueKind == JsonValueKind.True || element.ValueKind == JsonValueKind.False)
            {
                return element.GetBoolean();
            }

            // For non-boolean, try parsing from string if possible, else default
            if (element.ValueKind == JsonValueKind.String)
            {
                if (bool.TryParse(element.GetString(), out bool parsed))
                {
                    return parsed;
                }
            }

            return defaultValue;
        }

        public static int GetInt32(JsonElement payload, string property, int defaultValue = 0)
        {
            if (!payload.TryGetProperty(property, out JsonElement element))
            {
                return defaultValue;
            }

            return element.ValueKind switch
            {
                JsonValueKind.Number => element.GetInt32(),
                JsonValueKind.String => int.TryParse(element.GetString(), out int parsed) ? parsed : defaultValue,
                _ => defaultValue
            };
        }

        public static double GetDouble(JsonElement payload, string property, double defaultValue = 0.0)
        {
            if (!payload.TryGetProperty(property, out JsonElement element))
            {
                return defaultValue;
            }

            return element.ValueKind switch
            {
                JsonValueKind.Number => element.GetDouble(),
                JsonValueKind.String => double.TryParse(element.GetString(), out double parsed) ? parsed : defaultValue,
                _ => defaultValue
            };
        }

        public static string[]? GetStringArray(JsonElement payload, string property)
        {
            if (!payload.TryGetProperty(property, out JsonElement element))
            {
                return null;
            }

            if (element.ValueKind == JsonValueKind.Array)
            {
                var list = new List<string>();
                foreach (var item in element.EnumerateArray())
                {
                    if (item.ValueKind == JsonValueKind.String)
                    {
                        list.Add(item.GetString() ?? string.Empty);
                    }
                }
                return list.ToArray();
            }

            if (element.ValueKind == JsonValueKind.String)
            {
                try
                {
                    return JsonSerializer.Deserialize<string[]>(element.GetRawText()) ?? null;
                }
                catch
                {
                    return null;
                }
            }

            return null;
        }

        public static T? Deserialize<T>(JsonElement payload, string property) where T : class
        {
            if (!payload.TryGetProperty(property, out JsonElement element))
            {
                return null;
            }

            if (element.ValueKind == JsonValueKind.Object || element.ValueKind == JsonValueKind.Array)
            {
                try
                {
                    return JsonSerializer.Deserialize<T>(element.GetRawText());
                }
                catch (JsonException)
                {
                    return null;
                }
            }

            return null;
        }
    }
}
