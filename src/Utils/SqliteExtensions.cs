using Microsoft.Data.Sqlite;
using System.Collections.Concurrent;
using System.Linq.Expressions;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Maenifold.Utils;

public static class SqliteExtensions
{
    // CI environments (GitHub Actions, etc.) use tmpfs for /tmp which has issues with WAL mode.
    // Use DELETE journal mode in CI for reliability; WAL mode in production for performance.
    private static readonly bool IsCI = Environment.GetEnvironmentVariable("CI") == "true" ||
                                         Environment.GetEnvironmentVariable("GITHUB_ACTIONS") == "true";
    private static readonly string JournalMode = IsCI ? "DELETE" : "WAL";

    private static readonly ConcurrentDictionary<Type, Func<object>> _factoryCache = new();
    private static readonly ConcurrentDictionary<Type, PropertyInfo[]> _propertyCache = new();

    private static Func<object> GetOrCreateFactory(Type type)
    {
        return _factoryCache.GetOrAdd(type, t =>
        {
            var newExpr = Expression.New(t);
            var lambda = Expression.Lambda<Func<object>>(Expression.Convert(newExpr, typeof(object)));
            return lambda.Compile();
        });
    }

    public static void OpenWithWAL(this SqliteConnection conn)
    {
        conn.Open();
        conn.Execute("PRAGMA busy_timeout=" + Config.SqliteBusyTimeoutMs);
        conn.Execute($"PRAGMA journal_mode={JournalMode}");
        conn.Execute("PRAGMA synchronous=NORMAL");
    }

    public static void OpenReadOnly(this SqliteConnection conn)
    {
        conn.Open();
        conn.Execute("PRAGMA busy_timeout=" + Config.SqliteBusyTimeoutMs);
        conn.Execute($"PRAGMA journal_mode={JournalMode}");
        conn.Execute("PRAGMA query_only=ON");
        conn.Execute("PRAGMA read_uncommitted=ON");
    }

    public static void OpenReadOnlyWithVector(this SqliteConnection conn)
    {
        conn.Open();
        conn.Execute("PRAGMA busy_timeout=" + Config.SqliteBusyTimeoutMs);
        conn.Execute($"PRAGMA journal_mode={JournalMode}");
        conn.Execute("PRAGMA query_only=ON");
        conn.Execute("PRAGMA read_uncommitted=ON");
        conn.LoadVectorExtension();
    }

    public static void LoadVectorExtension(this SqliteConnection connection)
    {

        var rid = RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? "win-x64" :
                          RuntimeInformation.IsOSPlatform(OSPlatform.Linux) ? "linux-x64" :
                          RuntimeInformation.IsOSPlatform(OSPlatform.OSX) ?
                              (RuntimeInformation.ProcessArchitecture == Architecture.Arm64 ? "osx-arm64" : "osx-x64") :
                          throw new PlatformNotSupportedException();

        var ext = rid.StartsWithOrdinal("win") ? "dll" :
                          rid.StartsWithOrdinal("osx") ? "dylib" : "so";
        var fileName = $"vec0.{ext}";
        var path = Path.Combine(AppContext.BaseDirectory, "assets", "native", rid, fileName);
        connection.EnableExtensions(true);
        connection.LoadExtension(path);
    }

// CA2100: SQL strings are constructed internally, not from user input. Prepared statements used for all user-facing data.
#pragma warning disable CA2100
    public static void Execute(this SqliteConnection conn, string sql, object? param = null)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = sql;
        if (param != null)
        {
            foreach (var prop in param.GetType().GetProperties())
            {
                cmd.Parameters.AddWithValue($"@{prop.Name}", prop.GetValue(param) ?? DBNull.Value);
            }
        }
        cmd.ExecuteNonQuery();
    }

    public static T ExecuteScalar<T>(this SqliteConnection conn, string sql)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = sql;
        return (T)cmd.ExecuteScalar()!;
    }

    public static T? QuerySingle<T>(this SqliteConnection conn, string sql, object? param = null)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = sql;
        if (param != null)
        {
            foreach (var prop in param.GetType().GetProperties())
            {
                cmd.Parameters.AddWithValue($"@{prop.Name}", prop.GetValue(param) ?? DBNull.Value);
            }
        }
        using var reader = cmd.ExecuteReader();
        if (!reader.Read()) return default;


        if (typeof(T) == typeof(bool))
            return (T)(object)(reader.GetInt32(0) > 0);
        if (typeof(T) == typeof(int))
            return (T)(object)reader.GetInt32(0);
        if (typeof(T) == typeof(string))
            return (T)(object)reader.GetString(0);


        if (typeof(T).IsGenericType && typeof(T).GetGenericTypeDefinition() == typeof(ValueTuple<,,>))
        {
            var types = typeof(T).GetGenericArguments();
            var values = new object[types.Length];
            for (int i = 0; i < types.Length; i++)
            {
                values[i] = reader.IsDBNull(i) ? null! : reader.GetValue(i);
            }
            return (T)Activator.CreateInstance(typeof(T), values)!;
        }

        return default;
    }

    public static IEnumerable<T> Query<T>(this SqliteConnection conn, string sql, object? param = null)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = sql;
        if (param != null)
        {
            foreach (var prop in param.GetType().GetProperties())
            {
                cmd.Parameters.AddWithValue($"@{prop.Name}", prop.GetValue(param) ?? DBNull.Value);
            }
        }
        using var reader = cmd.ExecuteReader();
        var results = new List<T>();

        while (reader.Read())
        {

            if (typeof(T) == typeof(string))
            {
                results.Add((T)(object)reader.GetString(0));
            }

            else if (typeof(T) == typeof((string, int, string)))
            {
                var related = reader.GetString(0);
                var count = reader.GetInt32(1);
                var files = reader.GetString(2);
                results.Add((T)(object)(related, count, files));
            }

            else if (typeof(T) == typeof((string, string, int)))
            {
                var a = reader.GetString(0);
                var b = reader.GetString(1);
                var count = reader.GetInt32(2);
                results.Add((T)(object)(a, b, count));
            }

            else if (typeof(T).IsClass && !typeof(T).IsAbstract)
            {
                var factory = GetOrCreateFactory(typeof(T));
                var instance = (T)factory();
                var properties = _propertyCache.GetOrAdd(typeof(T), t => t.GetProperties());

                for (int i = 0; i < reader.FieldCount; i++)
                {
                    var columnName = reader.GetName(i);
                    var prop = properties.FirstOrDefault(p => p.Name.Equals(columnName, StringComparison.OrdinalIgnoreCase));
                    if (prop != null && prop.CanWrite && !reader.IsDBNull(i))
                    {
                        prop.SetValue(instance, reader.GetValue(i));
                    }
                }
                results.Add(instance);
            }
        }

        return results;
    }
#pragma warning restore CA2100
}
