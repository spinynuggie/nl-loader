using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace NeverloseLoader
{
    public class ConfigEntry
    {
        public int EntryId { get; set; }
        public string Name { get; set; }
    }

    public class LauncherSettings
    {
        public string Username { get; set; }
        public string AvatarDataUrl { get; set; }
        public int? SelectedConfigId { get; set; }
        public List<ConfigEntry> Configs { get; set; } = new List<ConfigEntry>();
    }

    public static class SettingsHelper
    {
        public static string GetLauncherCloudDir()
        {
            var envPath = Environment.GetEnvironmentVariable("NL_CLOUD_PATH");
            if (!string.IsNullOrEmpty(envPath)) return envPath;

            var csgoPath = SteamHelper.FindGameInstallPath("Counter-Strike Global Offensive");
            if (!string.IsNullOrEmpty(csgoPath))
            {
                var p = Path.Combine(csgoPath, "nl_cloud");
                if (Directory.Exists(p)) return p;
            }

            var csgoLegacyPath = SteamHelper.FindGameInstallPath("csgo legacy");
            if (!string.IsNullOrEmpty(csgoLegacyPath))
            {
                var p = Path.Combine(csgoLegacyPath, "nl_cloud");
                if (Directory.Exists(p)) return p;
            }

            if (!string.IsNullOrEmpty(csgoPath))
            {
                return Path.Combine(csgoPath, "nl_cloud");
            }
            if (!string.IsNullOrEmpty(csgoLegacyPath))
            {
                return Path.Combine(csgoLegacyPath, "nl_cloud");
            }

            var appData = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
            return Path.Combine(appData, "neverlose", "nl_cloud");
        }

        public static string SanitizeFilename(string name)
        {
            if (string.IsNullOrEmpty(name)) return "unnamed";
            var invalidChars = Path.GetInvalidFileNameChars();
            var sb = new StringBuilder();
            foreach (var c in name)
            {
                sb.Append(invalidChars.Contains(c) || char.IsControl(c) ? '_' : c);
            }
            var res = sb.ToString().TrimEnd('.', ' ');
            return string.IsNullOrEmpty(res) ? "unnamed" : res;
        }

        public static LauncherSettings ReadLauncherSettings()
        {
            var cloudDir = GetLauncherCloudDir();
            var statePath = Path.Combine(cloudDir, "state.json");

            var settings = new LauncherSettings
            {
                Username = "Player",
                AvatarDataUrl = null,
                SelectedConfigId = null,
                Configs = new List<ConfigEntry>()
            };

            if (!File.Exists(statePath))
            {
                return settings;
            }

            try
            {
                var text = File.ReadAllText(statePath);
                var state = JsonConvert.DeserializeObject<JObject>(text);
                if (state != null)
                {
                    settings.Username = state["username"]?.ToString() ?? "Player";
                    
                    if (state["last_loaded_config_id"] != null)
                    {
                        settings.SelectedConfigId = state["last_loaded_config_id"].ToObject<int?>();
                    }

                    var logToken = state["log"];
                    if (logToken is JArray logArray)
                    {
                        foreach (var entry in logArray)
                        {
                            var type = entry["entry_type"]?.ToString();
                            var deletedAt = entry["deleted_at"];
                            if (type == "Config" && (deletedAt == null || deletedAt.Type == JTokenType.Null))
                            {
                                settings.Configs.Add(new ConfigEntry
                                {
                                    EntryId = entry["entry_id"]?.ToObject<int>() ?? 0,
                                    Name = entry["name"]?.ToString() ?? "Unnamed"
                                });
                            }
                        }
                    }
                }
            }
            catch { }

            // Load Avatar
            var avatarPath = Path.Combine(cloudDir, "avatar.png");
            if (File.Exists(avatarPath))
            {
                try
                {
                    var bytes = File.ReadAllBytes(avatarPath);
                    var base64 = Convert.ToBase64String(bytes);
                    settings.AvatarDataUrl = $"data:image/png;base64,{base64}";
                }
                catch { }
            }

            return settings;
        }

        public static void SaveLauncherProfile(string username, byte[] avatarBytes)
        {
            username = username?.Trim();
            if (string.IsNullOrEmpty(username))
            {
                throw new ArgumentException("Profile name cannot be empty.");
            }

            var cloudDir = GetLauncherCloudDir();
            Directory.CreateDirectory(cloudDir);

            var statePath = Path.Combine(cloudDir, "state.json");
            JObject state = null;

            if (File.Exists(statePath))
            {
                try
                {
                    state = JsonConvert.DeserializeObject<JObject>(File.ReadAllText(statePath));
                }
                catch { }
            }

            if (state == null)
            {
                state = new JObject();
            }

            state["username"] = username;

            var prettyJson = JsonConvert.SerializeObject(state, Formatting.Indented);
            File.WriteAllText(statePath, prettyJson);

            if (avatarBytes != null && avatarBytes.Length > 0)
            {
                var avatarPath = Path.Combine(cloudDir, "avatar.png");
                File.WriteAllBytes(avatarPath, avatarBytes);
            }
        }
    }
}
