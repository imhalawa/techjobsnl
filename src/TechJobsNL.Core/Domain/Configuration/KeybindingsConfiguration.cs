namespace TechJobsNL.Core.Domain.Configuration;

/// <summary>Retains terminal action bindings without dispatching input.</summary>
public sealed record KeybindingsConfiguration
{
    /// <summary>Initializes a new instance of the <see cref="KeybindingsConfiguration"/> class.</summary>
    public KeybindingsConfiguration(string scan, string search, string filter, string toggleApplied, string history,
        string open, string copy, string help, string quit)
    {
        Scan = scan;
        Search = search;
        Filter = filter;
        ToggleApplied = toggleApplied;
        History = history;
        Open = open;
        Copy = copy;
        Help = help;
        Quit = quit;
    }

    /// <summary>Gets the Rust default for an omitted copy binding.</summary>
    public const string DefaultCopy = "c";

    public string Scan { get; }
    public string Search { get; }
    public string Filter { get; }
    public string ToggleApplied { get; }
    public string History { get; }
    public string Open { get; }
    public string Copy { get; }
    public string Help { get; }
    public string Quit { get; }
}
