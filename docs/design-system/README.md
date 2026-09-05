# TechJobsNL design system

## Status

Draft. This is the cross-client identity source. Release prototypes test it; accepted changes are made here rather than
forked inside one release.

## Product character

TechJobsNL is calm, precise, evidence-first, locally trustworthy, and useful at professional information density. It should
feel like a personal company-intelligence desk, not a social feed, recruitment marketplace, or AI chat wrapper.

## Experience principles

1. **Follow first:** orient the experience around companies a person deliberately follows.
2. **Evidence nearby:** make origin, recency, confidence, and the path to source material easy to inspect.
3. **Deterministic first:** every essential journey works without AiExperience.
4. **Progressive depth:** show a useful summary first and let the person inspect detail without losing context.
5. **Quiet state:** distinguish new, unread, changed, failed, and uncertain information without alarm fatigue.
6. **Local trust:** explain network activity, retained data, external links, and optional model use at the point of action.
7. **Shared meaning:** desktop, terminal, and mobile may differ visually while preserving canonical terms and states.

## Identity system

- **Primary ink:** deep navy `#0B1220` for durable structure and high-attention text.
- **Primary action:** clear blue `#2563EB` for navigation, selection, and ordinary action.
- **Intelligence accent:** teal `#0F766E` for evidence, technology, and deterministic intelligence.
- **AiExperience accent:** violet `#7C3AED`, always paired with an explicit label rather than color alone.
- **Success:** green `#15803D`; **warning/incomplete:** amber `#B45309`; **failure:** red `#B91C1C`.
- **Surfaces:** neutral slate steps from white/light gray to deep slate; content remains readable without decorative color.
- **Typography:** platform UI sans-serif for prose and controls; platform monospace for source IDs, code, and technical facts.
- **Spacing:** a 4-pixel base scale; prefer `4, 8, 12, 16, 24, 32, 48`.
- **Shape:** 6-pixel default radius, restrained borders, and shadows only when they communicate elevation.
- **Motion:** brief and functional; respect reduced-motion preferences and avoid movement for routine Feed arrival.

## Required references

- [`patterns.md`](patterns.md): shared Feed, profile, evidence, state, and AiExperience patterns.
- [`prototypes.md`](prototypes.md): the prototype contract for every release.
