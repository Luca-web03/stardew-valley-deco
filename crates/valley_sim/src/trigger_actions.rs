//! Trigger actions (`Data/TriggerActions`, public format) and the action
//! strings they run.
//!
//! Entries (`Id`, `Trigger`, `Condition`, `Action` / `Actions`,
//! `MarkActionApplied` default true) run when their trigger is raised
//! (`DayStarted`, `DayEnding`, `LocationChanged`) and their condition
//! passes; applied entries are remembered and don't run again.
//!
//! Actions: `AddBuff`/`RemoveBuff`, `AddConversationTopic`,
//! `RemoveConversationTopic`, `AddFriendshipPoints`, `AddItem`,
//! `RemoveItem`, `AddMail` (`now`, `tomorrow` default, `received`),
//! `RemoveMail`, `AddMoney`, `AddQuest`, `RemoveQuest`, `AddSpecialOrder`,
//! `RemoveSpecialOrder`, `If <query> ## <action> [## <action>]`,
//! `IncrementStat`, `MarkActionApplied`, `MarkCookingRecipeKnown`,
//! `MarkCraftingRecipeKnown`, `MarkEventSeen`, `MarkQuestionAnswered`,
//! `MarkSongHeard`, `Null`, `RemoveTemporaryAnimatedSprites`,
//! `SetNpcInvisible`, `SetNpcVisible`. A player argument (`Current`,
//! `Host`, `All`...) addresses the current player.

use serde::Deserialize;

use crate::world::World;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct TriggerActionData {
    pub id: String,
    pub trigger: String,
    pub condition: Option<String>,
    pub host_only: bool,
    pub action: Option<String>,
    pub actions: Vec<String>,
    pub mark_action_applied: Option<bool>,
}

pub fn load(cm: &mut valley_content::ContentManager) -> Vec<TriggerActionData> {
    cm.load_json("Data/TriggerActions").unwrap_or_default()
}

fn is_player(s: &str) -> bool {
    matches!(s.to_ascii_lowercase().as_str(), "current" | "host" | "all" | "any" | "target") || s.starts_with("PlayerId")
}

fn flag(args: &[String], i: usize) -> bool {
    args.get(i).map_or(true, |v| !v.eq_ignore_ascii_case("false"))
}

impl World {
    /// Raise a trigger: run the matching `Data/TriggerActions` entries.
    pub fn raise_trigger(&mut self, trigger: &str) {
        let entries: Vec<TriggerActionData> = self.trigger_actions.iter().filter(|e| e.trigger.split_whitespace().any(|t| t.eq_ignore_ascii_case(trigger))).cloned().collect();
        for e in entries {
            if self.farmer.trigger_actions_run.contains(&e.id) {
                continue;
            }
            let mut rng = self.day_rng(4321);
            if !self.check_query(e.condition.as_deref(), None, None, &mut rng) {
                continue;
            }
            for a in e.action.iter().chain(e.actions.iter()) {
                if let Err(err) = self.run_game_action(a) {
                    eprintln!("trigger action {}: {err}", e.id);
                }
            }
            if e.mark_action_applied.unwrap_or(true) && !self.farmer.trigger_actions_run.contains(&e.id) {
                self.farmer.trigger_actions_run.push(e.id.clone());
            }
        }
    }

    /// Run one action string.
    pub fn run_game_action(&mut self, action: &str) -> Result<(), String> {
        let action = action.trim();
        if let Some(rest) = action.strip_prefix("If ") {
            let parts: Vec<&str> = rest.split("##").map(str::trim).collect();
            let mut rng = self.day_rng(4322);
            let ok = self.check_query(Some(parts.first().copied().unwrap_or("")), None, None, &mut rng);
            return match (ok, parts.get(1), parts.get(2)) {
                (true, Some(a), _) => self.run_game_action(a),
                (false, _, Some(a)) => self.run_game_action(a),
                _ => Ok(()),
            };
        }
        let toks = crate::gsq::tokens(action);
        let Some(name) = toks.first().cloned() else { return Ok(()) };
        // Drop a leading player argument where actions take one.
        let mut args: Vec<String> = toks[1..].to_vec();
        let takes_player = matches!(
            name.as_str(),
            "AddMail" | "RemoveMail" | "MarkActionApplied" | "MarkCookingRecipeKnown" | "MarkCraftingRecipeKnown" | "MarkEventSeen" | "MarkQuestionAnswered" | "MarkSongHeard" | "AddCookingRecipe" | "AddCraftingRecipe"
        );
        if takes_player && args.first().is_some_and(|a| is_player(a)) {
            args.remove(0);
        }
        let arg = |i: usize| args.get(i).cloned().ok_or_else(|| format!("{name}: missing argument {i}"));
        let int = |i: usize, d: i32| args.get(i).and_then(|v| v.parse().ok()).unwrap_or(d);
        let f = &mut self.farmer;
        match name.as_str() {
            "Null" | "RemoveTemporaryAnimatedSprites" => {}
            "AddMail" => {
                let id = arg(0)?;
                match args.get(1).map(|t| t.to_ascii_lowercase()).as_deref() {
                    Some("now") => {
                        if !f.mailbox.contains(&id) {
                            f.mailbox.push(id);
                        }
                    }
                    Some("received") => {
                        if !f.mail_received.contains(&id) {
                            f.mail_received.push(id);
                        }
                    }
                    _ => {
                        if !f.mail_tomorrow.contains(&id) {
                            f.mail_tomorrow.push(id);
                        }
                    }
                }
            }
            "RemoveMail" => {
                let id = arg(0)?;
                let kind = args.get(1).map(|t| t.to_ascii_lowercase()).unwrap_or_else(|| "all".into());
                if kind == "all" || kind == "now" || kind == "mailbox" {
                    f.mailbox.retain(|m| *m != id);
                }
                if kind == "all" || kind == "tomorrow" {
                    f.mail_tomorrow.retain(|m| *m != id);
                }
                if kind == "all" || kind == "received" {
                    f.mail_received.retain(|m| *m != id);
                }
            }
            "AddMoney" => {
                let n = int(0, 0);
                f.money = (f.money + n).max(0);
                if n > 0 {
                    f.total_earned += n as i64;
                }
            }
            "AddConversationTopic" => {
                f.conversation_topics.insert(arg(0)?, int(1, 4));
            }
            "RemoveConversationTopic" => {
                f.conversation_topics.remove(&arg(0)?);
            }
            "AddFriendshipPoints" => {
                let npc = arg(0)?;
                let n = int(1, 0);
                let fr = f.friendship(&npc);
                fr.points = (fr.points + n).max(0);
            }
            "IncrementStat" => {
                *f.counters.entry(arg(0)?).or_insert(0) += int(1, 1);
            }
            "MarkActionApplied" => {
                let id = arg(0)?;
                f.trigger_actions_run.retain(|a| *a != id);
                if flag(&args, 1) {
                    f.trigger_actions_run.push(id);
                }
            }
            "MarkEventSeen" => {
                let id = arg(0)?;
                f.events_seen.retain(|a| *a != id);
                if flag(&args, 1) {
                    f.events_seen.push(id);
                }
            }
            "MarkQuestionAnswered" => {
                let id = arg(0)?;
                f.dialogue_answers.retain(|a| *a != id);
                if flag(&args, 1) {
                    f.dialogue_answers.push(id);
                }
            }
            "MarkSongHeard" => {
                let key = format!("Port.SongHeard.{}", arg(0)?);
                if flag(&args, 1) {
                    f.counters.insert(key, 1);
                } else {
                    f.counters.remove(&key);
                }
            }
            "MarkCookingRecipeKnown" | "AddCookingRecipe" => {
                let r = args.iter().take_while(|a| !a.eq_ignore_ascii_case("true") && !a.eq_ignore_ascii_case("false")).cloned().collect::<Vec<_>>().join(" ");
                if args.last().is_some_and(|a| a.eq_ignore_ascii_case("false")) {
                    f.cooking_recipes.remove(&r);
                } else {
                    f.cooking_recipes.entry(r).or_insert(0);
                }
            }
            "MarkCraftingRecipeKnown" | "AddCraftingRecipe" => {
                let r = args.iter().take_while(|a| !a.eq_ignore_ascii_case("true") && !a.eq_ignore_ascii_case("false")).cloned().collect::<Vec<_>>().join(" ");
                if args.last().is_some_and(|a| a.eq_ignore_ascii_case("false")) {
                    f.crafting_recipes.remove(&r);
                } else {
                    f.crafting_recipes.entry(r).or_insert(0);
                }
            }
            "AddItem" => {
                let id = arg(0)?;
                let mut item = self.items.create(&id, int(1, 1)).ok_or_else(|| format!("AddItem: unknown item {id}"))?;
                if let Some(q) = args.get(2).and_then(|q| q.parse().ok()) {
                    if let crate::item::Item::Object { quality, .. } = &mut item {
                        *quality = crate::item::Quality::from_level(q);
                    }
                }
                self.give_item(item);
            }
            "RemoveItem" => {
                let id = arg(0)?;
                let (_, raw) = crate::item::split_qualified(&id);
                f.inventory.remove_object(raw, int(1, 1));
            }
            "AddQuest" => {
                self.add_quest(&arg(0)?);
            }
            "RemoveQuest" => {
                let id = arg(0)?;
                f.quests.retain(|q| q.id != id);
            }
            "AddSpecialOrder" => {
                self.accept_order(&arg(0)?)?;
            }
            "RemoveSpecialOrder" => {
                let id = arg(0)?;
                if f.special_order.as_ref().is_some_and(|o| o.id == id) {
                    f.special_order = None;
                }
            }
            "AddBuff" => {
                // Buffs defined on objects (by object id).
                let id = arg(0)?;
                let (_, raw) = crate::item::split_qualified(&id);
                let info = self.items.object(raw).cloned().ok_or_else(|| format!("AddBuff: unknown buff {id}"))?;
                let buffs = info.data.buffs.as_ref().map(|b| crate::buffs::parse_buffs(b, &info.display_name)).unwrap_or_default();
                if buffs.is_empty() {
                    return Err(format!("AddBuff: {id} has no buff"));
                }
                for mut b in buffs {
                    if let Some(m) = args.get(1).and_then(|v| v.parse::<i32>().ok()) {
                        b.minutes_left = m;
                    }
                    self.farmer.buffs.retain(|x| x.id != b.id);
                    self.farmer.buffs.push(b);
                }
            }
            "RemoveBuff" => {
                let id = arg(0)?;
                f.buffs.retain(|b| b.id != id && b.source != id);
            }
            "SetNpcInvisible" => {
                let npc = arg(0)?;
                f.counters.insert(format!("Port.Invisible.{npc}"), int(1, 1).max(1));
                self.hide_invisible_npcs();
            }
            "SetNpcVisible" => {
                let npc = arg(0)?;
                f.counters.remove(&format!("Port.Invisible.{npc}"));
                if let Some(n) = self.npcs.iter_mut().find(|n| n.name == npc) {
                    if n.location.is_empty() {
                        n.location = n.home.location.clone();
                        n.set_tile(n.home.tile);
                    }
                }
            }
            other => return Err(format!("unknown action {other}")),
        }
        Ok(())
    }

    /// NPCs made invisible are nowhere to be found.
    pub(crate) fn hide_invisible_npcs(&mut self) {
        for n in self.npcs.iter_mut() {
            if self.farmer.counters.get(&format!("Port.Invisible.{}", n.name)).is_some_and(|d| *d > 0) {
                n.location.clear();
            }
        }
    }

    /// Morning: invisibility wears off day by day; letters sent "tomorrow"
    /// arrive.
    pub(crate) fn trigger_actions_morning(&mut self) {
        let mail: Vec<String> = std::mem::take(&mut self.farmer.mail_tomorrow);
        for m in mail {
            if !self.farmer.mailbox.contains(&m) {
                self.farmer.mailbox.push(m);
            }
        }
        let keys: Vec<String> = self.farmer.counters.keys().filter(|k| k.starts_with("Port.Invisible.")).cloned().collect();
        for k in keys {
            let left = self.farmer.counters.get(&k).copied().unwrap_or(0) - 1;
            if left <= 0 {
                self.farmer.counters.remove(&k);
            } else {
                self.farmer.counters.insert(k, left);
            }
        }
        self.hide_invisible_npcs();
    }
}
