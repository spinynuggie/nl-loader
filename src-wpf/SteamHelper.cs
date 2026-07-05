using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading;
using Microsoft.Win32;

namespace NeverloseLoader
{
    public class InstalledGames
    {
        public bool Cs2LegacyBranch { get; set; }
        public bool CsgoStandalone { get; set; }
    }

    public static class SteamHelper
    {
        private const string CsgoWindowClass = "Valve001";
        private const uint WmClose = 0x0010;

        [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
        private static extern IntPtr FindWindow(string lpClassName, string lpWindowName);

        [DllImport("user32.dll", SetLastError = true)]
        private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);

        [DllImport("user32.dll", CharSet = CharSet.Auto)]
        private static extern IntPtr PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);

        public static string GetSteamInstallPath()
        {
            try
            {
                using (var key = Registry.CurrentUser.OpenSubKey(@"Software\Valve\Steam"))
                {
                    var val = key?.GetValue("SteamPath")?.ToString();
                    if (!string.IsNullOrEmpty(val)) return val;
                }
            }
            catch { }

            try
            {
                using (var key = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\Wow6432Node\Valve\Steam"))
                {
                    var val = key?.GetValue("InstallPath")?.ToString();
                    if (!string.IsNullOrEmpty(val)) return val;
                }
            }
            catch { }

            try
            {
                using (var key = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\Valve\Steam"))
                {
                    var val = key?.GetValue("InstallPath")?.ToString();
                    if (!string.IsNullOrEmpty(val)) return val;
                }
            }
            catch { }

            return null;
        }

        public static string FindGameInstallPath(string gameName)
        {
            // Check current directory first
            var curDir = Directory.GetCurrentDirectory();
            if (File.Exists(Path.Combine(curDir, "csgo.exe")))
            {
                return curDir;
            }

            var steamPath = GetSteamInstallPath();
            if (string.IsNullOrEmpty(steamPath)) return null;

            Func<string, string, string> checkPath = (lib, name) =>
            {
                var fullPath = Path.Combine(lib, "steamapps", "common", name);
                if (File.Exists(Path.Combine(fullPath, "csgo.exe")))
                {
                    return fullPath;
                }
                return null;
            };

            Func<string, string> checkAllLibs = (name) =>
            {
                var p = checkPath(steamPath, name);
                if (p != null) return p;

                var vdfPath = Path.Combine(steamPath, "steamapps", "libraryfolders.vdf");
                if (File.Exists(vdfPath))
                {
                    try
                    {
                        var content = File.ReadAllText(vdfPath);
                        var libPaths = ExtractLibraryPaths(content);
                        foreach (var libPath in libPaths)
                        {
                            p = checkPath(libPath, name);
                            if (p != null) return p;
                        }
                    }
                    catch { }
                }
                return null;
            };

            var path = checkAllLibs(gameName);
            if (path != null) return path;

            if (gameName == "Counter-Strike Global Offensive")
            {
                path = checkAllLibs("Counter-Strike Global Offensive 730");
                if (path != null) return path;
            }

            return null;
        }

        private static List<string> ExtractLibraryPaths(string content)
        {
            var paths = new List<string>();
            // Matches "path" "[whitespace]" "[value]"
            var regex = new Regex(@"\""path\""\s+\""([^\""]+)\""", RegexOptions.IgnoreCase);
            var matches = regex.Matches(content);
            foreach (Match match in matches)
            {
                if (match.Groups.Count > 1)
                {
                    var p = match.Groups[1].Value.Replace(@"\\", @"\");
                    if (Directory.Exists(p))
                    {
                        paths.Add(p);
                    }
                }
            }
            return paths;
        }

        public static InstalledGames DetectInstalledGames()
        {
            return new InstalledGames
            {
                Cs2LegacyBranch = FindGameInstallPath("Counter-Strike Global Offensive") != null,
                CsgoStandalone = FindGameInstallPath("csgo legacy") != null
            };
        }

        public static void CloseCsgoIfRunning()
        {
            var window = FindWindow(CsgoWindowClass, null);
            if (window == IntPtr.Zero) return;

            GetWindowThreadProcessId(window, out var processId);
            PostMessage(window, WmClose, IntPtr.Zero, IntPtr.Zero);

            if (processId == 0)
            {
                Thread.Sleep(1400);
                return;
            }

            try
            {
                var proc = Process.GetProcessById((int)processId);
                if (!proc.WaitForExit(3500))
                {
                    proc.Kill();
                    proc.WaitForExit(2000);
                }
            }
            catch { }

            Thread.Sleep(700);
        }

        public static void RestartCsgo(int appid)
        {
            CloseCsgoIfRunning();

            var steamPath = GetSteamInstallPath();
            if (string.IsNullOrEmpty(steamPath))
            {
                throw new Exception("Failed to find Steam install path.");
            }

            var steamExe = Path.Combine(steamPath, "steam.exe");
            if (!File.Exists(steamExe))
            {
                throw new Exception("Steam executable not found.");
            }

            string protocolString = appid == 730 
                ? "steam://launch/730/option2" 
                : $"steam://launch/{appid}/dialog";

            var startInfo = new ProcessStartInfo
            {
                FileName = steamExe,
                Arguments = $"{protocolString} -steam -insecure -novid",
                WorkingDirectory = steamPath,
                UseShellExecute = true
            };

            Process.Start(startInfo);
        }
    }
}
