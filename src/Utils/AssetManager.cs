using System.ComponentModel;
using ModelContextProtocol.Server;

namespace Maenifold.Utils;

[McpServerResourceType]
public static class AssetManager
{
    public static void InitializeAssets()
    {
        var sourceAssetsPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "assets");
        var targetAssetsPath = Config.AssetsPath;

        if (!Directory.Exists(sourceAssetsPath))
        {

            return;
        }


        Directory.CreateDirectory(targetAssetsPath);

        CopyAssetsRecursive(sourceAssetsPath, targetAssetsPath);
    }

    public static AssetUpdateResult UpdateAssets(bool dryRun = true)
    {
        var sourceAssetsPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "assets");
        var targetAssetsPath = Config.AssetsPath;
        var result = new AssetUpdateResult();

        if (!Directory.Exists(sourceAssetsPath))
        {
            result.Error = "Source assets directory not found";
            return result;
        }

        if (!Directory.Exists(targetAssetsPath))
        {
            result.Error = "Target assets directory does not exist. Run initialization first.";
            return result;
        }

        try
        {
            CompareAndUpdateAssets(sourceAssetsPath, targetAssetsPath, dryRun, result);
        }
        catch (Exception ex)
        {
            result.Error = $"Update failed: {ex.Message}";
        }

        return result;
    }

    private static void CompareAndUpdateAssets(string sourceDir, string targetDir, bool dryRun, AssetUpdateResult result)
    {
        if (!Directory.Exists(targetDir))
        {
            Directory.CreateDirectory(targetDir);
        }

        foreach (var sourceFile in Directory.GetFiles(sourceDir))
        {
            var fileName = Path.GetFileName(sourceFile);
            var targetFile = Path.Combine(targetDir, fileName);

            if (!File.Exists(targetFile))
            {
                result.AddedFiles.Add(targetFile);
                if (!dryRun)
                {
                    try
                    {
                        File.Copy(sourceFile, targetFile);
                    }
                    catch (Exception ex)
                    {
                        result.Errors.Add($"Failed to copy new file {fileName}: {ex.Message}");
                    }
                }
            }
            else
            {
                var sourceInfo = new FileInfo(sourceFile);
                var targetInfo = new FileInfo(targetFile);
                if (sourceInfo.Length != targetInfo.Length ||
                    !FilesAreIdentical(sourceFile, targetFile))
                {
                    result.UpdatedFiles.Add(targetFile);
                    if (!dryRun)
                    {
                        try
                        {
                            File.Copy(sourceFile, targetFile, overwrite: true);
                        }
                        catch (Exception ex)
                        {
                            result.Errors.Add($"Failed to update file {fileName}: {ex.Message}");
                        }
                    }
                }
            }
        }

        foreach (var sourceSubDir in Directory.GetDirectories(sourceDir))
        {
            var dirName = Path.GetFileName(sourceSubDir);
            var targetSubDir = Path.Combine(targetDir, dirName);
            CompareAndUpdateAssets(sourceSubDir, targetSubDir, dryRun, result);
        }
    }

    private static bool FilesAreIdentical(string file1, string file2)
    {
        try
        {
            var hash1 = GetFileHash(file1);
            var hash2 = GetFileHash(file2);
            return hash1.SequenceEqual(hash2);
        }
        catch
        {
            return false;
        }
    }

    private static byte[] GetFileHash(string filePath)
    {
        using (var sha256 = System.Security.Cryptography.SHA256.Create())
        using (var stream = File.OpenRead(filePath))
        {
            return sha256.ComputeHash(stream);
        }
    }

    private static void CopyAssetsRecursive(string sourceDir, string targetDir)
    {

        Directory.CreateDirectory(targetDir);


        foreach (var sourceFile in Directory.GetFiles(sourceDir))
        {
            var fileName = Path.GetFileName(sourceFile);
            var targetFile = Path.Combine(targetDir, fileName);

            if (!File.Exists(targetFile))
            {
                File.Copy(sourceFile, targetFile);

            }
        }


        foreach (var sourceSubDir in Directory.GetDirectories(sourceDir))
        {
            var dirName = Path.GetFileName(sourceSubDir);
            var targetSubDir = Path.Combine(targetDir, dirName);
            CopyAssetsRecursive(sourceSubDir, targetSubDir);
        }
    }

    public static string GetAssetPath(string relativePath)
    {
        return Path.Combine(Config.AssetsPath, relativePath);
    }

    public static bool AssetExists(string relativePath)
    {
        return File.Exists(GetAssetPath(relativePath));
    }

    public static string? LoadAssetText(string relativePath)
    {
        var path = GetAssetPath(relativePath);
        return File.Exists(path) ? File.ReadAllText(path) : null;
    }

    public static T? LoadAssetJson<T>(string relativePath) where T : class
    {
        var text = LoadAssetText(relativePath);
        return text != null ? System.Text.Json.JsonSerializer.Deserialize<T>(text) : null;
    }

}

public class AssetUpdateResult
{
    public List<string> AddedFiles { get; } = new();
    public List<string> UpdatedFiles { get; } = new();
    public List<string> Errors { get; } = new();
    public string? Error { get; set; }

    public override string ToString()
    {
        var sb = new System.Text.StringBuilder("# Asset Update Summary\n\n");

        if (!string.IsNullOrEmpty(Error))
        {
            sb.AppendLine("## Error");
            sb.AppendLine(Error);
            return sb.ToString();
        }

        sb.AppendLineInvariant($"## Results\n- Added files: {AddedFiles.Count}");
        sb.AppendLineInvariant($"- Updated files: {UpdatedFiles.Count}");
        sb.AppendLineInvariant($"- Errors: {Errors.Count}");

        if (AddedFiles.Count > 0)
        {
            sb.AppendLine("\n### Added Files");
            foreach (var file in AddedFiles.OrderBy(f => f))
            {
                var relativePath = Path.GetRelativePath(Config.AssetsPath, file);
                sb.AppendLineInvariant($"- {relativePath}");
            }
        }

        if (UpdatedFiles.Count > 0)
        {
            sb.AppendLine("\n### Updated Files");
            foreach (var file in UpdatedFiles.OrderBy(f => f))
            {
                var relativePath = Path.GetRelativePath(Config.AssetsPath, file);
                sb.AppendLineInvariant($"- {relativePath}");
            }
        }

        if (Errors.Count > 0)
        {
            sb.AppendLine("\n### Errors");
            foreach (var error in Errors)
            {
                sb.AppendLineInvariant($"- {error}");
            }
        }

        return sb.ToString();
    }
}