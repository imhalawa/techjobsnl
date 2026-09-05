namespace TechJobsNL.Core.Domain.Configuration;

/// <summary>Retains optional UI color tokens.</summary>
public sealed record ThemeOverrides
{
    /// <summary>Initializes a new instance of the <see cref="ThemeOverrides"/> class.</summary>
    public ThemeOverrides(string? background, string? focusedBorder, string? unfocusedBorder, string? selectedRow,
        string? primaryText, string? mutedText, string? open, string? @new, string? applied, string? warning, string? error)
    {
        Background = background;
        FocusedBorder = focusedBorder;
        UnfocusedBorder = unfocusedBorder;
        SelectedRow = selectedRow;
        PrimaryText = primaryText;
        MutedText = mutedText;
        Open = open;
        New = @new;
        Applied = applied;
        Warning = warning;
        Error = error;
    }

    /// <summary>Gets an empty set of overrides.</summary>
    public static ThemeOverrides Empty { get; } = new(null, null, null, null, null, null, null, null, null, null, null);

    public string? Background { get; }
    public string? FocusedBorder { get; }
    public string? UnfocusedBorder { get; }
    public string? SelectedRow { get; }
    public string? PrimaryText { get; }
    public string? MutedText { get; }
    public string? Open { get; }
    public string? New { get; }
    public string? Applied { get; }
    public string? Warning { get; }
    public string? Error { get; }
}
