using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Microsoft.Win32;
using Wpf.Ui.Controls;
using Wpf.Ui.Appearance;
using System.Windows.Media.Animation;

namespace NeverloseLoader
{
    public partial class MainWindow : FluentWindow
    {
        private LauncherGitMetadata _gitMetadata;
        private LauncherSettings _settings;
        private byte[] _avatarBytes;

        public MainWindow()
        {
            SystemThemeWatcher.Watch(this);
            InitializeComponent();
        }

        private async void NavView_Loaded(object sender, RoutedEventArgs e)
        {
            NavView.Navigate("launcher");
            await InitializeLauncherAsync();
        }

        private async Task InitializeLauncherAsync()
        {
            UpdateSystemStatus();
            LoadSettings();
            await LoadGitMetadataAsync();
        }

        private async Task ShowDialogAsync(string title, string content, string closeButtonText = "Ok")
        {
            var dialog = new ContentDialog(RootContentDialogHost)
            {
                Title = title,
                Content = content,
                CloseButtonText = closeButtonText
            };
            await dialog.ShowAsync();
        }

        private void UpdateSystemStatus()
        {
            // Detect Installed Games
            var status = SteamHelper.DetectInstalledGames();
            
            // Populate the ComboGame dynamically
            var availableGames = new List<GameItem>();
            if (status.Cs2LegacyBranch)
            {
                availableGames.Add(new GameItem { Name = "csgo-cs2-legacy (730)", AppId = 730 });
            }
            if (status.CsgoStandalone)
            {
                availableGames.Add(new GameItem { Name = "csgo-standalone (4465480)", AppId = 4465480 });
            }

            // Fallback: if no games detected, populate both so user is not blocked
            if (availableGames.Count == 0)
            {
                availableGames.Add(new GameItem { Name = "csgo-cs2-legacy (730)", AppId = 730 });
                availableGames.Add(new GameItem { Name = "csgo-standalone (4465480)", AppId = 4465480 });
            }

            ComboGame.ItemsSource = availableGames;
            ComboGame.DisplayMemberPath = "Name";
            ComboGame.SelectedValuePath = "AppId";

            if (availableGames.Count > 0)
            {
                ComboGame.SelectedIndex = 0;
                ComboGame.IsEnabled = true;
                BtnLaunch.IsEnabled = true;
            }
            else
            {
                ComboGame.IsEnabled = false;
                BtnLaunch.IsEnabled = false;
            }
        }

        private void LoadSettings()
        {
            _settings = SettingsHelper.ReadLauncherSettings();

            // Update sidebar profile widget
            TxtUsername.Text = _settings.Username;
            if (BtnProfileWidget?.Template != null)
            {
                BtnProfileWidget.ApplyTemplate();
                if (BtnProfileWidget.Template.FindName("WidgetUsername", BtnProfileWidget) is System.Windows.Controls.TextBlock tb)
                    tb.Text = _settings.Username;
                if (BtnProfileWidget.Template.FindName("WidgetAvatar", BtnProfileWidget) is System.Windows.Controls.Image img)
                    img.Source = !string.IsNullOrEmpty(_settings.AvatarDataUrl) ? LoadBase64Image(_settings.AvatarDataUrl) : null;
            }

            // Set Avatar (hidden ref)
            UpdateAvatarDisplay(_settings.AvatarDataUrl);

            // Populate Configs
            ComboConfig.ItemsSource = _settings.Configs;
            ComboConfig.DisplayMemberPath = "Name";
            ComboConfig.SelectedValuePath = "EntryId";

            if (_settings.SelectedConfigId.HasValue)
            {
                ComboConfig.SelectedValue = _settings.SelectedConfigId.Value;
            }
            else if (_settings.Configs.Count > 0)
            {
                ComboConfig.SelectedIndex = 0;
            }
        }

        private void UpdateAvatarDisplay(string dataUrl)
        {
            if (!string.IsNullOrEmpty(dataUrl))
            {
                var bmp = LoadBase64Image(dataUrl);
                if (bmp != null)
                {
                    ImgAvatar.Source = bmp;
                    return;
                }
            }

            // Fallback placeholder
            ImgAvatar.Source = null;
        }

        private BitmapImage LoadBase64Image(string dataUrl)
        {
            try
            {
                var commaIndex = dataUrl.IndexOf(',');
                if (commaIndex >= 0)
                {
                    var base64 = dataUrl.Substring(commaIndex + 1);
                    var bytes = Convert.FromBase64String(base64);
                    using (var ms = new MemoryStream(bytes))
                    {
                        var image = new BitmapImage();
                        image.BeginInit();
                        image.CacheOption = BitmapCacheOption.OnLoad;
                        image.StreamSource = ms;
                        image.EndInit();
                        image.Freeze();
                        return image;
                    }
                }
            }
            catch { }
            return null;
        }

        private async Task LoadGitMetadataAsync()
        {
            try
            {
                _gitMetadata = await GitHubHelper.LoadGitMetadataAsync();
                UpdateVersionsList();
            }
            catch (Exception ex)
            {
                TxtStatus.Text = "Failed to load version metadata from GitHub.";
                await ShowDialogAsync("Network Error", $"Failed to fetch Git releases:\n{ex.Message}");
            }
        }

        private void UpdateVersionsList()
        {
            if (_gitMetadata == null) return;

            var list = ToggleNightly.IsChecked == true 
                ? _gitMetadata.Nightlies 
                : _gitMetadata.Releases;

            ComboVersion.ItemsSource = list;
            ComboVersion.DisplayMemberPath = "Name";
            ComboVersion.SelectedValuePath = "Tag";

            if (list.Count > 0)
            {
                ComboVersion.SelectedIndex = 0;
            }
            else
            {
                ComboVersion.ItemsSource = null;
            }
        }

        private void Branch_Checked(object sender, RoutedEventArgs e)
        {
            UpdateVersionsList();
        }

        private void NavView_SelectionChanged(object sender, RoutedEventArgs e)
        {
            if (sender is NavigationView navView && navView.SelectedItem is NavigationViewItem item)
            {
                var tag = item.TargetPageTag ?? item.Tag?.ToString();
                if (tag != null && tag != "profile-widget")
                    ShowView(tag);
            }
        }

        private void ShowView(string viewTag)
        {
            LauncherView.Visibility = Visibility.Collapsed;
            ChangelogView.Visibility = Visibility.Collapsed;

            var target = viewTag == "launcher" ? LauncherView : LauncherView;
            target.Visibility = Visibility.Visible;
            ApplyCustomTransition(target, 350);
        }

        private void ComboVersion_SelectionChanged(object sender, SelectionChangedEventArgs e)
        {
            if (ComboVersion.SelectedItem is LauncherVersion selectedVersion)
            {
                TxtChangelogTitle.Text = $"Changelog for {selectedVersion.Name}";
                TxtChangelogSubTitle.Text = $"Changelog - {selectedVersion.Name}";
                RenderMarkdownToTextBlock(TxtChangelogContent, selectedVersion.Changelog);
            }
            else
            {
                TxtChangelogTitle.Text = "Select a version to see release notes";
                TxtChangelogSubTitle.Text = "Changelog";
                RenderMarkdownToTextBlock(TxtChangelogContent, "Select a version to see its release notes.");
            }
        }

        private void BtnViewChangelog_Click(object sender, RoutedEventArgs e)
        {
            LauncherView.Visibility = Visibility.Collapsed;
            ChangelogView.Visibility = Visibility.Visible;
            ApplyCustomTransition(ChangelogView, 300);
        }

        private void BtnBackToLauncher_Click(object sender, RoutedEventArgs e)
        {
            ChangelogView.Visibility = Visibility.Collapsed;
            LauncherView.Visibility = Visibility.Visible;
            ApplyCustomTransition(LauncherView, 300);
        }

        private void ApplyCustomTransition(UIElement element, int durationMs)
        {
            var timespan = TimeSpan.FromMilliseconds(durationMs);
            
            var fadeAnim = new DoubleAnimation
            {
                From = 0.0,
                To = 1.0,
                Duration = timespan,
                EasingFunction = new CubicEase { EasingMode = EasingMode.EaseOut }
            };
            
            var transform = new TranslateTransform();
            element.RenderTransform = transform;
            
            var slideAnim = new DoubleAnimation
            {
                From = 8,
                To = 0,
                Duration = timespan,
                EasingFunction = new CubicEase { EasingMode = EasingMode.EaseOut }
            };
            
            element.BeginAnimation(UIElement.OpacityProperty, fadeAnim);
            transform.BeginAnimation(TranslateTransform.YProperty, slideAnim);
        }

        private async void BtnLaunch_Click(object sender, RoutedEventArgs e)
        {
            if (!(ComboVersion.SelectedValue is string selectedTag))
            {
                await ShowDialogAsync("Validation Error", "Please select a version to download and launch.");
                return;
            }

            if (!(ComboGame.SelectedValue is int appid))
            {
                await ShowDialogAsync("Validation Error", "Please select a target game to launch.");
                return;
            }

            int? configId = ComboConfig.SelectedValue as int?;

            // Disable launch UI
            BtnLaunch.IsEnabled = false;
            ComboGame.IsEnabled = false;
            ComboConfig.IsEnabled = false;
            ComboVersion.IsEnabled = false;
            ToggleNightly.IsEnabled = false;
            ProgressLaunchRing.Visibility = Visibility.Visible;

            try
            {
                await GitHubHelper.DownloadAndLaunchVersionAsync(
                    selectedTag,
                    configId,
                    appid,
                    (statusText, percentage) =>
                    {
                        Dispatcher.Invoke(() =>
                        {
                            TxtStatus.Text = statusText;
                            ProgressLaunch.Value = percentage;
                        });
                    }
                );

                TxtStatus.Text = "Launch triggered successfully!";
            }
            catch (Exception ex)
            {
                TxtStatus.Text = "Launch failed.";
                await ShowDialogAsync("Execution Error", $"An error occurred during download/launch:\n{ex.Message}");
            }
            finally
            {
                // Re-enable UI
                BtnLaunch.IsEnabled = true;
                ComboGame.IsEnabled = true;
                ComboConfig.IsEnabled = true;
                ComboVersion.IsEnabled = true;
                ToggleNightly.IsEnabled = true;
                ProgressLaunchRing.Visibility = Visibility.Collapsed;
            }
        }

        private void ComboGame_SelectionChanged(object sender, SelectionChangedEventArgs e)
        {
        }

        private async void BtnProfileWidget_Click(object sender, RoutedEventArgs e)
        {
            _avatarBytes = null;

            // Build dialog content
            var usernameBox = new Wpf.Ui.Controls.TextBox
            {
                Width = 200,
                PlaceholderText = "Enter display name...",
                Text = _settings.Username
            };

            var avatarPreview = new System.Windows.Controls.Image { Stretch = Stretch.UniformToFill };
            if (!string.IsNullOrEmpty(_settings.AvatarDataUrl))
                avatarPreview.Source = LoadBase64Image(_settings.AvatarDataUrl);

            var avatarBorder = new Border
            {
                CornerRadius = new CornerRadius(20),
                Width = 40,
                Height = 40,
                ClipToBounds = true,
                Margin = new Thickness(0, 0, 12, 0),
                VerticalAlignment = VerticalAlignment.Center,
                Child = avatarPreview
            };

            var changeAvatarBtn = new Wpf.Ui.Controls.Button { Content = "Change Avatar", VerticalAlignment = VerticalAlignment.Center };
            changeAvatarBtn.Click += (s, _) =>
            {
                var ofd = new OpenFileDialog { Filter = "Image Files|*.png;*.jpg;*.jpeg;*.gif;*.webp", Title = "Select Profile Image" };
                if (ofd.ShowDialog() == true)
                {
                    try
                    {
                        _avatarBytes = File.ReadAllBytes(ofd.FileName);
                        var bmp = LoadBase64Image($"data:image/png;base64,{Convert.ToBase64String(_avatarBytes)}");
                        avatarPreview.Source = bmp;
                    }
                    catch { }
                }
            };

            var avatarRow = new StackPanel { Orientation = Orientation.Horizontal, VerticalAlignment = VerticalAlignment.Center };
            avatarRow.Children.Add(avatarBorder);
            avatarRow.Children.Add(changeAvatarBtn);

            var nameCard = new Wpf.Ui.Controls.CardControl
            {
                Margin = new Thickness(0, 0, 0, 8),
                Icon = new Wpf.Ui.Controls.SymbolIcon { Symbol = Wpf.Ui.Controls.SymbolRegular.Person24 },
                Header = new StackPanel { Children =
                {
                    new System.Windows.Controls.TextBlock { Text = "Display Name", FontWeight = FontWeights.SemiBold },
                    new System.Windows.Controls.TextBlock { Text = "Change your nickname", FontSize = 12, Foreground = (Brush)FindResource("SystemControlPageTextBaseMediumBrush") }
                }},
                Content = usernameBox
            };

            var avatarCard = new Wpf.Ui.Controls.CardControl
            {
                Margin = new Thickness(0, 0, 0, 8),
                Icon = new Wpf.Ui.Controls.SymbolIcon { Symbol = Wpf.Ui.Controls.SymbolRegular.Image24 },
                Header = new StackPanel { Children =
                {
                    new System.Windows.Controls.TextBlock { Text = "Profile Picture", FontWeight = FontWeights.SemiBold },
                    new System.Windows.Controls.TextBlock { Text = "Change your account avatar", FontSize = 12, Foreground = (Brush)FindResource("SystemControlPageTextBaseMediumBrush") }
                }},
                Content = avatarRow
            };

            var panel = new StackPanel { Margin = new Thickness(0, 10, 0, 10) };
            panel.Children.Add(nameCard);
            panel.Children.Add(avatarCard);

            var dialog = new ContentDialog(RootContentDialogHost)
            {
                Title = "Edit Profile",
                Content = panel,
                PrimaryButtonText = "Save",
                CloseButtonText = "Cancel"
            };

            dialog.Closing += async (d, args) =>
            {
                if (args.Result == ContentDialogResult.Primary)
                {
                    var username = usernameBox.Text.Trim();
                    if (string.IsNullOrEmpty(username))
                    {
                        args.Cancel = true;
                        await ShowDialogAsync("Validation Error", "Display name cannot be empty.");
                        return;
                    }
                    try
                    {
                        SettingsHelper.SaveLauncherProfile(username, _avatarBytes);
                        _avatarBytes = null;
                        LoadSettings();
                    }
                    catch (Exception ex)
                    {
                        args.Cancel = true;
                        await ShowDialogAsync("Error", $"Failed to save profile:\n{ex.Message}");
                    }
                }
            };

            await dialog.ShowAsync();
        }



        private void RenderMarkdownToTextBlock(System.Windows.Controls.TextBlock textBlock, string markdownText)
        {
            textBlock.Inlines.Clear();
            if (string.IsNullOrEmpty(markdownText))
            {
                textBlock.Text = "";
                return;
            }

            var lines = markdownText.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.None);
            for (int i = 0; i < lines.Length; i++)
            {
                var line = lines[i];
                var trimmed = line.Trim();
                if (trimmed.StartsWith("##"))
                {
                    // H2 header
                    var run = new Run(trimmed.Substring(2).Trim())
                    {
                        FontSize = 15,
                        FontWeight = FontWeights.Bold,
                        Foreground = (Brush)FindResource("SystemControlPageTextBaseHighBrush")
                    };
                    textBlock.Inlines.Add(run);
                    if (i < lines.Length - 1) { textBlock.Inlines.Add(new LineBreak()); textBlock.Inlines.Add(new LineBreak()); }
                }
                else if (trimmed.StartsWith("#"))
                {
                    // H1 header
                    var run = new Run(trimmed.Substring(1).Trim())
                    {
                        FontSize = 17,
                        FontWeight = FontWeights.Bold,
                        Foreground = (Brush)FindResource("SystemControlPageTextBaseHighBrush")
                    };
                    textBlock.Inlines.Add(run);
                    if (i < lines.Length - 1) { textBlock.Inlines.Add(new LineBreak()); textBlock.Inlines.Add(new LineBreak()); }
                }
                else if (trimmed.StartsWith("-") || trimmed.StartsWith("*"))
                {
                    // Bullet list item
                    textBlock.Inlines.Add(new Run("  •  ") { FontWeight = FontWeights.Bold, Foreground = (Brush)FindResource("SystemControlPageTextBaseHighBrush") });
                    ParseLineWithLinks(textBlock, trimmed.Substring(1).Trim());
                    if (i < lines.Length - 1) { textBlock.Inlines.Add(new LineBreak()); textBlock.Inlines.Add(new LineBreak()); }
                }
                else
                {
                    if (!string.IsNullOrEmpty(trimmed))
                    {
                        ParseLineWithLinks(textBlock, trimmed);
                        if (i < lines.Length - 1) { textBlock.Inlines.Add(new LineBreak()); textBlock.Inlines.Add(new LineBreak()); }
                    }
                }
            }
        }

        private void ParseLineWithLinks(System.Windows.Controls.TextBlock textBlock, string text)
        {
            int currentIndex = 0;
            while (currentIndex < text.Length)
            {
                int linkStart = text.IndexOf('[', currentIndex);
                if (linkStart < 0)
                {
                    textBlock.Inlines.Add(new Run(text.Substring(currentIndex)));
                    break;
                }

                if (linkStart > currentIndex)
                {
                    textBlock.Inlines.Add(new Run(text.Substring(currentIndex, linkStart - currentIndex)));
                }

                int linkEnd = text.IndexOf(']', linkStart);
                if (linkEnd < 0)
                {
                    textBlock.Inlines.Add(new Run(text.Substring(linkStart)));
                    break;
                }

                int urlStart = text.IndexOf('(', linkEnd);
                if (urlStart != linkEnd + 1)
                {
                    textBlock.Inlines.Add(new Run(text.Substring(linkStart, linkEnd - linkStart + 1)));
                    currentIndex = linkEnd + 1;
                    continue;
                }

                int urlEnd = text.IndexOf(')', urlStart);
                if (urlEnd < 0)
                {
                    textBlock.Inlines.Add(new Run(text.Substring(linkStart)));
                    break;
                }

                var linkText = text.Substring(linkStart + 1, linkEnd - linkStart - 1);
                var url = text.Substring(urlStart + 1, urlEnd - urlStart - 1);

                try
                {
                    var hyperlink = new Hyperlink(new Run(linkText))
                    {
                        NavigateUri = new Uri(url),
                        Cursor = Cursors.Arrow,
                        TextDecorations = null
                    };

                    hyperlink.MouseEnter += (s, e) =>
                    {
                        hyperlink.Foreground = new SolidColorBrush(Color.FromRgb(0, 90, 158));
                        hyperlink.TextDecorations = TextDecorations.Underline;
                    };
                    hyperlink.MouseLeave += (s, e) =>
                    {
                        hyperlink.ClearValue(Hyperlink.ForegroundProperty);
                        hyperlink.TextDecorations = null;
                    };

                    hyperlink.RequestNavigate += (s, e) =>
                    {
                        try
                        {
                            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
                            {
                                FileName = e.Uri.AbsoluteUri,
                                UseShellExecute = true
                            });
                        }
                        catch { }
                        e.Handled = true;
                    };

                    textBlock.Inlines.Add(hyperlink);
                }
                catch
                {
                    textBlock.Inlines.Add(new Run(text.Substring(linkStart, urlEnd - linkStart + 1)));
                }

                currentIndex = urlEnd + 1;
            }
        }
    }

    public class GameItem
    {
        public string Name { get; set; }
        public int AppId { get; set; }
    }

    public class DummyPage : System.Windows.Controls.Page
    {
    }
}
