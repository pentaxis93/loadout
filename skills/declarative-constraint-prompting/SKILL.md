---
name: declarative-constraint-prompting
description: Transform any input into a declarative constraint prompt — an XML specification that maximizes signal, eliminates implementation bias, and produces testable requirements an agent can execute without clarification.
---

```xml
<skill>
  <goal>
    Transform any input — problem statements, feature requests, vague ideas,
    conversation transcripts, existing prompts — into a declarative constraint
    prompt formatted as XML inside a markdown code fence.
  </goal>

  <output-spec>
    <required-elements>
      <goal>Single sentence stating what the system or output must be</goal>
      <constraints>
        Hard boundaries the solution must satisfy.
        Facts grounded in research, not assumptions.
      </constraints>
      <requirements>
        Observable, testable properties the result must exhibit.
        Each requirement passes a yes/no verification by an implementer.
      </requirements>
    </required-elements>
    <conditional-elements>
      <non-goals>
        Explicit scope exclusions. Include only when ambiguity
        exists about what is in or out of scope.
      </non-goals>
      <context>
        Domain facts the implementer needs but might not know.
        Include only when the domain is specialized.
      </context>
      <examples>
        Concrete input/output pairs. Include only when the spec
        alone leaves room for misinterpretation.
      </examples>
    </conditional-elements>
  </output-spec>

  <principles>
    <principle name="declarative-only">
      Describe WHAT is wanted. Never HOW to achieve it.
      No algorithms, implementation hints, or technology choices
      unless the technology itself is a stated constraint.
    </principle>
    <principle name="signal-over-noise">
      Every element must earn its place. Remove adjectives, hedging,
      motivation, and backstory. If it does not change what gets built,
      it does not belong.
    </principle>
    <principle name="clean-slate">
      Ignore the input's current implementation unless explicitly
      marked as a hard constraint. Reframe around first principles.
      If the input says "improve X," determine what X must accomplish
      and specify that instead.
    </principle>
    <principle name="grounded-constraints">
      When the input references tools, APIs, formats, or standards,
      research them before writing constraints. Constraints must
      reflect verified reality.
    </principle>
    <principle name="testable-requirements">
      Each requirement must be verifiable without reading the
      requester's mind. "Easy to use" fails. "Single command to
      list all items with status" passes.
    </principle>
    <principle name="minimal-completeness">
      Include everything needed to build the right thing.
      Exclude everything else.
    </principle>
  </principles>

  <interaction>
    <clear-input>Produce the spec directly.</clear-input>
    <ambiguous-input>
      Ask the minimum questions needed to resolve ambiguity.
      Prefer producing a spec with explicit assumptions stated
      over extended question-and-answer.
    </ambiguous-input>
    <existing-prompt>
      Rewrite it. Strip implementation details, surface implicit
      constraints, make requirements testable, add non-goals
      where scope is ambiguous.
    </existing-prompt>
  </interaction>
</skill>
```
