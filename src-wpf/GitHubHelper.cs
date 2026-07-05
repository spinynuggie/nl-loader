using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Net.Http;
using System.Security.Cryptography;
using System.Text;
using System.Threading.Tasks;
using Newtonsoft.Json;

namespace NeverloseLoader
{
    public class GithubAsset
    {
        [JsonProperty("name")]
        public string Name { get; set; }

        [JsonProperty("browser_download_url")]
        public string BrowserDownloadUrl { get; set; }

        [JsonProperty("size")]
        public long Size { get; set; }
    }

    public class GithubRelease
    {
        [JsonProperty("tag_name")]
        public string TagName { get; set; }

        [JsonProperty("name")]
        public string Name { get; set; }

        [JsonProperty("body")]
        public string Body { get; set; }

        [JsonProperty("prerelease")]
        public bool Prerelease { get; set; }

        [JsonProperty("draft")]
        public bool Draft { get; set; }

        [JsonProperty("published_at")]
        public string PublishedAt { get; set; }

        [JsonProperty("updated_at")]
        public string UpdatedAt { get; set; }

        [JsonProperty("html_url")]
        public string HtmlUrl { get; set; }

        [JsonProperty("assets")]
        public List<GithubAsset> Assets { get; set; }
    }

    public class LauncherAsset
    {
        public string Name { get; set; }
        public string Url { get; set; }
        public long Size { get; set; }
    }

    public class LauncherVersion
    {
        public string Tag { get; set; }
        public string Name { get; set; }
        public string Changelog { get; set; }
        public string UpdatedAt { get; set; }
        public string Url { get; set; }
        public List<LauncherAsset> Assets { get; set; } = new List<LauncherAsset>();
    }

    public class LauncherGitMetadata
    {
        public List<LauncherVersion> Releases { get; set; } = new List<LauncherVersion>();
        public List<LauncherVersion> Nightlies { get; set; } = new List<LauncherVersion>();
    }

    public static class GitHubHelper
    {
        private const string GithubReleasesUrl = "https://api.github.com/repos/luvettee/FemboyLose/releases";
        private const string GithubReleaseByTagUrl = "https://api.github.com/repos/luvettee/FemboyLose/releases/tags";

        private static HttpClient CreateClient()
        {
            var client = new HttpClient();
            client.DefaultRequestHeaders.Add("User-Agent", "NeverloseWPFLauncher");
            return client;
        }

        public static async Task<LauncherGitMetadata> LoadGitMetadataAsync()
        {
            using (var client = CreateClient())
            {
                var response = await client.GetAsync(GithubReleasesUrl);
                response.EnsureSuccessStatusCode();

                var json = await response.Content.ReadAsStringAsync();
                var rawReleases = JsonConvert.DeserializeObject<List<GithubRelease>>(json);

                var metadata = new LauncherGitMetadata();

                foreach (var r in rawReleases)
                {
                    if (r.Draft) continue;

                    var version = new LauncherVersion
                    {
                        Tag = r.TagName,
                        Name = string.IsNullOrEmpty(r.Name) ? r.TagName : r.Name,
                        Changelog = r.Body ?? "",
                        UpdatedAt = !string.IsNullOrEmpty(r.PublishedAt) ? r.PublishedAt : r.UpdatedAt,
                        Url = r.HtmlUrl,
                        Assets = r.Assets.Select(a => new LauncherAsset
                        {
                            Name = a.Name,
                            Url = a.BrowserDownloadUrl,
                            Size = a.Size
                        }).ToList()
                    };

                    if (r.Prerelease)
                    {
                        metadata.Nightlies.Add(version);
                    }
                    else
                    {
                        metadata.Releases.Add(version);
                    }
                }

                return metadata;
            }
        }

        public static string GetVersionInstallDir(string tag)
        {
            var appData = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
            return Path.Combine(appData, "neverlose", "bin", SettingsHelper.SanitizeFilename(tag));
        }

        public static void KillBackgroundProcesses()
        {
            try
            {
                var cmd1 = new ProcessStartInfo("taskkill", "/im injector.exe /f")
                {
                    CreateNoWindow = true,
                    UseShellExecute = false,
                    WindowStyle = ProcessWindowStyle.Hidden
                };
                Process.Start(cmd1)?.WaitForExit(1000);
            }
            catch { }

            try
            {
                var cmd2 = new ProcessStartInfo("taskkill", "/im neverlose-server.exe /f")
                {
                    CreateNoWindow = true,
                    UseShellExecute = false,
                    WindowStyle = ProcessWindowStyle.Hidden
                };
                Process.Start(cmd2)?.WaitForExit(1000);
            }
            catch { }
        }

        public static async Task DownloadAndLaunchVersionAsync(
            string tag,
            int? configId,
            int appid,
            Action<string, double> progressCallback)
        {
            if (string.IsNullOrWhiteSpace(tag) || tag == "Unavailable")
            {
                throw new ArgumentException("No release version is selected.");
            }

            progressCallback("Fetching release details...", 5);

            GithubRelease release;
            using (var client = CreateClient())
            {
                var response = await client.GetAsync($"{GithubReleaseByTagUrl}/{tag}");
                response.EnsureSuccessStatusCode();
                var json = await response.Content.ReadAsStringAsync();
                release = JsonConvert.DeserializeObject<GithubRelease>(json);
            }

            var installDir = GetVersionInstallDir(tag);
            Directory.CreateDirectory(installDir);

            progressCallback("Killing background processes...", 15);
            await Task.Run(() => KillBackgroundProcesses());

            // Clean up old files
            try
            {
                foreach (var file in Directory.GetFiles(installDir, "*.old"))
                {
                    File.Delete(file);
                }
            }
            catch { }

            // Download Checksums if available
            progressCallback("Downloading checksums...", 20);
            var checksums = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            var sumsAsset = release.Assets.FirstOrDefault(a => string.Equals(a.Name, "SHA256SUMS.txt", StringComparison.OrdinalIgnoreCase));
            if (sumsAsset != null)
            {
                try
                {
                    using (var client = CreateClient())
                    {
                        var text = await client.GetStringAsync(sumsAsset.BrowserDownloadUrl);
                        foreach (var line in text.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.RemoveEmptyEntries))
                        {
                            var parts = line.Split(new[] { ' ', '\t' }, StringSplitOptions.RemoveEmptyEntries);
                            if (parts.Length >= 2)
                            {
                                var hash = parts[0].Trim().ToLower();
                                var filename = parts[1].Trim().TrimStart('*');
                                checksums[filename] = hash;
                            }
                        }
                    }
                }
                catch { }
            }

            // Download required assets
            var requiredAssets = new[] { "neverlose.dll", "neverlose-server.exe", "injector.exe" };
            double assetProgressStep = 60.0 / requiredAssets.Length;

            for (int i = 0; i < requiredAssets.Length; i++)
            {
                var assetName = requiredAssets[i];
                double currentProgress = 20.0 + (i * assetProgressStep);
                progressCallback($"Downloading {assetName}...", currentProgress);
                await DownloadAssetWithRetryAsync(release, assetName, installDir, checksums, progressCallback, currentProgress, assetProgressStep);
            }

            progressCallback("Configuring game parameters...", 85);
            var gameFolderName = appid == 730 ? "Counter-Strike Global Offensive" : "csgo legacy";
            var gameDir = SteamHelper.FindGameInstallPath(gameFolderName);
            var cloudDir = !string.IsNullOrEmpty(gameDir) 
                ? Path.Combine(gameDir, "nl_cloud") 
                : SettingsHelper.GetLauncherCloudDir();

            Directory.CreateDirectory(cloudDir);

            progressCallback("Launching Neverlose server...", 90);
            var serverExe = Path.Combine(installDir, "neverlose-server.exe");
            await Task.Run(() => SpawnServerHidden(serverExe, installDir, cloudDir, configId));

            progressCallback("Launching Injector...", 95);
            var injectorExe = Path.Combine(installDir, "injector.exe");
            if (IsLegacyVersion(tag))
            {
                await Task.Run(() => SteamHelper.RestartCsgo(appid));
                await Task.Run(() => SpawnInjectorLegacy(injectorExe, installDir));
            }
            else
            {
                var headlessChoice = appid == 730 ? 2 : 1;
                await Task.Run(() => SpawnInjectorHeadless(injectorExe, installDir, headlessChoice, gameDir));
            }

            progressCallback("Done!", 100);
        }

        private static async Task DownloadAssetWithRetryAsync(
            GithubRelease release,
            string assetName,
            string installDir,
            Dictionary<string, string> checksums,
            Action<string, double> progressCallback,
            double baseProgress,
            double progressWeight)
        {
            var targetPath = Path.Combine(installDir, assetName);

            // Check if file already exists and hash matches
            if (File.Exists(targetPath) && checksums.TryGetValue(assetName, out var expectedHash))
            {
                var actualHash = await Task.Run(() => CalculateFileSha256(targetPath));
                if (string.Equals(actualHash, expectedHash, StringComparison.OrdinalIgnoreCase))
                {
                    return; // Skip download
                }
            }

            var asset = release.Assets.FirstOrDefault(a => string.Equals(a.Name, assetName, StringComparison.OrdinalIgnoreCase));
            if (asset == null)
            {
                throw new Exception($"Release {release.TagName} is missing {assetName}");
            }

            var tempPath = Path.Combine(installDir, $"{assetName}.download");

            using (var client = CreateClient())
            using (var response = await client.GetAsync(asset.BrowserDownloadUrl, HttpCompletionOption.ResponseHeadersRead))
            {
                response.EnsureSuccessStatusCode();
                var totalBytes = response.Content.Headers.ContentLength ?? asset.Size;

                using (var stream = await response.Content.ReadAsStreamAsync())
                using (var fileStream = new FileStream(tempPath, FileMode.Create, FileAccess.Write, FileShare.None, 8192, true))
                {
                    var buffer = new byte[8192];
                    long totalRead = 0;
                    int bytesRead;

                    while ((bytesRead = await stream.ReadAsync(buffer, 0, buffer.Length)) != 0)
                    {
                        await fileStream.WriteAsync(buffer, 0, bytesRead);
                        totalRead += bytesRead;

                        if (totalBytes > 0)
                        {
                            double ratio = (double)totalRead / totalBytes;
                            progressCallback($"Downloading {assetName} ({Math.Round(ratio * 100)}%)...", baseProgress + (ratio * progressWeight * 0.9));
                        }
                    }
                }
            }

            // Verify hash of downloaded file
            if (checksums.TryGetValue(assetName, out var expected))
            {
                var hash = await Task.Run(() => CalculateFileSha256(tempPath));
                if (!string.Equals(hash, expected, StringComparison.OrdinalIgnoreCase))
                {
                    try { File.Delete(tempPath); } catch { }
                    throw new Exception($"Checksum validation failed for {assetName}. Expected {expected}, got {hash}");
                }
            }

            // Safely rename or backup existing file
            if (File.Exists(targetPath))
            {
                try
                {
                    File.Delete(targetPath);
                }
                catch
                {
                    var epoch = (long)(DateTime.UtcNow - new DateTime(1970, 1, 1)).TotalSeconds;
                    var backupPath = Path.Combine(installDir, $"{assetName}.{epoch}.old");
                    File.Move(targetPath, backupPath);
                }
            }

            File.Move(tempPath, targetPath);
        }

        private static string CalculateFileSha256(string path)
        {
            using (var sha = SHA256.Create())
            using (var stream = File.OpenRead(path))
            {
                var hashBytes = sha.ComputeHash(stream);
                var sb = new StringBuilder();
                foreach (var b in hashBytes)
                {
                    sb.Append(b.ToString("x2"));
                }
                return sb.ToString();
            }
        }

        private static void SpawnServerHidden(string exe, string workingDir, string cloudDir, int? configId)
        {
            var startInfo = new ProcessStartInfo
            {
                FileName = exe,
                WorkingDirectory = workingDir,
                CreateNoWindow = true,
                UseShellExecute = false,
                WindowStyle = ProcessWindowStyle.Hidden
            };

            startInfo.EnvironmentVariables["NL_CLOUD_PATH"] = cloudDir;
            if (configId.HasValue)
            {
                startInfo.Arguments = $"--boot-config {configId.Value}";
            }

            Process.Start(startInfo);
        }

        private static void SpawnInjectorHeadless(string exe, string workingDir, int choice, string gameDir)
        {
            var startInfo = new ProcessStartInfo
            {
                FileName = exe,
                WorkingDirectory = workingDir,
                CreateNoWindow = true,
                UseShellExecute = false,
                WindowStyle = ProcessWindowStyle.Hidden
            };

            startInfo.EnvironmentVariables["INJECTOR_HEADLESS"] = choice.ToString();
            if (!string.IsNullOrEmpty(gameDir))
            {
                startInfo.EnvironmentVariables["INJECTOR_GAME_DIR"] = gameDir;
            }

            Process.Start(startInfo);
        }

        private static void SpawnInjectorLegacy(string exe, string workingDir)
        {
            var startInfo = new ProcessStartInfo
            {
                FileName = exe,
                WorkingDirectory = workingDir,
                CreateNoWindow = true,
                UseShellExecute = false,
                WindowStyle = ProcessWindowStyle.Hidden
            };

            Process.Start(startInfo);
        }

        public static bool IsLegacyVersion(string tag)
        {
            if (string.IsNullOrEmpty(tag)) return false;
            var match = System.Text.RegularExpressions.Regex.Match(tag, @"[vV]?(\d+)\.(\d+)(?:\.(\d+))?");
            if (match.Success)
            {
                int major = int.Parse(match.Groups[1].Value);
                int minor = int.Parse(match.Groups[2].Value);
                if (major < 1) return true;
                if (major == 1 && minor < 1) return true;
            }
            return false;
        }
    }
}
