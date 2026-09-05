namespace TechJobsNL.Core.Application.Dispatch;

/// <summary>Exposes the separate command and query dispatch entry points.</summary>
public sealed class Dispatchers
{
    internal Dispatchers(ICommandDispatcher commands, IQueryDispatcher queries)
    {
        Commands = commands;
        Queries = queries;
    }

    /// <summary>Gets the command dispatcher.</summary>
    public ICommandDispatcher Commands { get; }

    /// <summary>Gets the query dispatcher.</summary>
    public IQueryDispatcher Queries { get; }
}
