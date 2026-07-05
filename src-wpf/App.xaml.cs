using System.Windows;
using Wpf.Ui.Appearance;

namespace NeverloseLoader
{
    public partial class App : Application
    {
        protected override void OnStartup(StartupEventArgs e)
        {
            ApplicationThemeManager.Apply(ApplicationTheme.Dark);
            base.OnStartup(e);
        }
    }
}
