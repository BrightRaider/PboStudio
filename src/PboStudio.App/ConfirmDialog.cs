using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using PboStudio.Core;

namespace PboStudio.App;

/// <summary>
/// Minimal modal confirmation. Avalonia ships no message box, and the one place that
/// genuinely needs one - setting every core to the chip's absolute limit - is exactly the
/// place where a mis-click costs the user a bluescreen.
/// </summary>
public static class ConfirmDialog
{
    public static async Task<bool> ShowAsync(
        Window owner, string title, string message, string confirmText, string cancelText, bool destructive = true)
    {
        bool result = false;

        var dialog = new Window
        {
            Title = title,
            Width = 460,
            SizeToContent = SizeToContent.Height,
            CanResize = false,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
            ShowInTaskbar = false,
            RequestedThemeVariant = Avalonia.Styling.ThemeVariant.Dark,
            Background = SolidColorBrush.Parse("#0B0D12"),
        };

        var confirmButton = new Button
        {
            Content = confirmText,
            MinWidth = 130,
            Padding = new Thickness(14, 7),
        };
        confirmButton.Classes.Add(destructive ? "danger" : "primary");
        confirmButton.Click += (_, _) => { result = true; dialog.Close(); };

        var cancelButton = new Button
        {
            Content = cancelText,
            MinWidth = 100,
            Padding = new Thickness(14, 7),
            IsDefault = true,
        };
        cancelButton.Classes.Add("secondary");
        cancelButton.Click += (_, _) => { result = false; dialog.Close(); };

        var card = new Border
        {
            Padding = new Thickness(18),
            Child = new StackPanel
            {
                Spacing = 12,
                Children =
                {
                    new StackPanel
                    {
                        Orientation = Orientation.Horizontal,
                        Spacing = 10,
                        Children =
                        {
                            new TextBlock { Text = destructive ? "⚠️" : "❔", FontSize = 20, VerticalAlignment = VerticalAlignment.Center },
                            new TextBlock
                            {
                                Text = title,
                                FontSize = 15,
                                FontWeight = FontWeight.Bold,
                                Foreground = SolidColorBrush.Parse(destructive ? "#F87171" : "#F8FAFC"),
                                VerticalAlignment = VerticalAlignment.Center,
                            },
                        },
                    },
                    new TextBlock
                    {
                        Text = message,
                        FontSize = 12.5,
                        LineHeight = 19,
                        TextWrapping = TextWrapping.Wrap,
                        Foreground = SolidColorBrush.Parse("#CBD5E1"),
                    },
                    new StackPanel
                    {
                        Orientation = Orientation.Horizontal,
                        Spacing = 8,
                        HorizontalAlignment = HorizontalAlignment.Right,
                        Margin = new Thickness(0, 4, 0, 0),
                        Children = { cancelButton, confirmButton },
                    },
                },
            },
        };
        card.Classes.Add("card");

        dialog.Content = card;
        // Escape must cancel, never confirm.
        dialog.KeyDown += (_, e) =>
        {
            if (e.Key == Avalonia.Input.Key.Escape) { result = false; dialog.Close(); }
        };

        await dialog.ShowDialog(owner);
        return result;
    }

    public static Task<bool> ShowAsync(Window owner, string message, string confirmText) =>
        ShowAsync(owner, LocalizationService.Get("ConfirmTitle"), message,
                  confirmText, LocalizationService.Get("ConfirmNo"));
}
