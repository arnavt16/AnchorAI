# 60–90 second showcase script

For demoing Anchor once it's built and Ollama is set up on your machine.
Turn off networking (or just note that Anchor works this way) before
starting, to make the offline claim visible rather than asserted.

1. **(0:00)** Open Anchor. Point out the vault location shown in Settings — a plain file on this computer, nothing cloud about it.
2. **(0:10)** Write a new entry: an interview worry. Track it as a worry with an expected outcome. Save.
3. **(0:20)** Go to the demo vault (Settings → toggle it on) instead of waiting for real history to accumulate — it seeds 20 fictional entries including a resolved interview-anxiety worry with a linked outcome.
4. **(0:30)** Go to Reflect, ask about interview nerves, pick "Look for a related experience." Show the response citing the fictional past entry and its recorded outcome.
5. **(0:50)** Open the source drawer — click through to see the actual retrieved passage and date, not just the model's paraphrase.
6. **(1:00)** Go to a worry with a *mixed or unfavorable* recorded outcome (e.g. the internship rejection fixture) and reflect on something related. Point out the response doesn't rewrite the bad outcome as reassurance — this is the detail that's easy to get wrong and worth calling out explicitly.
7. **(1:15)** In Settings, toggle memory off for one entry. Reflect again on something that would have matched it, and show it's gone from the sources.
8. **(1:25)** Restart the app. Show the entry, worry, and outcome are still there — canonical data survived the restart; only the session chat did not (open Reflect again and note the conversation reset).

Total: ~90 seconds. The two moments worth lingering on if you have more
time: the linked-outcome expansion (step 4) and the honest-unfavorable-
outcome behavior (step 6) — those are the two things a generic RAG demo
over raw text usually gets wrong or doesn't attempt at all.
