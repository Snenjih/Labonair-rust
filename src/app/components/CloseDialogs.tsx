import type React from "react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { handleApiError } from "@/lib/errors";
import type { EditorPaneHandle } from "@/modules/editor";
import type { PendingConfirmation } from "@/modules/tabs/lib/closeConfirmation";

export interface CloseDialogsProps {
  pendingConfirmations: PendingConfirmation[];
  setPendingConfirmations: React.Dispatch<React.SetStateAction<PendingConfirmation[]>>;
  disposeTab: (id: number) => void;
  editorRefs: React.MutableRefObject<Map<number, EditorPaneHandle>>;
}

export function CloseDialogs({
  pendingConfirmations,
  setPendingConfirmations,
  disposeTab,
  editorRefs,
}: CloseDialogsProps) {
  const current = pendingConfirmations[0] ?? null;
  // `AlertDialogAction`/`AlertDialogCancel` are Radix `Dialog.Close`s under
  // the hood — clicking either one auto-closes the dialog (firing
  // `onOpenChange(false)`) unless the click handler calls
  // `preventDefault()`. So `onOpenChange` is the ONE place that shifts the
  // queue for every synchronous action (Don't Save / Close Anyway / Close /
  // Cancel) — an onClick handler must never *also* call shift(), or a
  // single click would drop two queued tabs instead of one. The async Save
  // action is the sole exception: it preventDefaults so the dialog doesn't
  // close before the save resolves, and shifts manually only on success.
  const shift = () => setPendingConfirmations((q) => q.slice(1));

  return (
    <>
      <AlertDialog open={current?.kind === "save"} onOpenChange={(open) => !open && shift()}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Save before closing?</AlertDialogTitle>
            <AlertDialogDescription>
              "{current?.kind === "save" ? current.title : ""}" has not been saved.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel
              onClick={() => {
                if (current?.kind === "save") disposeTab(current.id);
              }}
            >
              Don't Save
            </AlertDialogCancel>
            <AlertDialogAction
              onClick={(e) => {
                if (current?.kind !== "save") return;
                const tab = current;
                // Always prevent the default sync-close — the save is
                // async, and a failure must leave the dialog open instead
                // of vanishing (Radix would otherwise close it immediately,
                // before the awaited save() even settles).
                e.preventDefault();
                void (async () => {
                  const h = editorRefs.current.get(tab.id);
                  try {
                    if (h) await h.save();
                    disposeTab(tab.id);
                    shift();
                  } catch (err) {
                    handleApiError(err, "Failed to save file", "Editor");
                    // Leave the dialog open so the user can retry Save or
                    // choose Don't Save instead of it silently vanishing.
                  }
                })();
              }}
            >
              Save
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      <AlertDialog open={current?.kind === "dirty"} onOpenChange={(open) => !open && shift()}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Close with unsaved changes?</AlertDialogTitle>
            <AlertDialogDescription>
              "{current?.kind === "dirty" ? current.title : ""}" has unsaved changes. They will be lost.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              onClick={() => {
                if (current?.kind === "dirty") disposeTab(current.id);
              }}
            >
              Close Anyway
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      <AlertDialog open={current?.kind === "terminal"} onOpenChange={(open) => !open && shift()}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Close terminal tab?</AlertDialogTitle>
            <AlertDialogDescription>The running shell process will be terminated.</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                if (current?.kind === "terminal") disposeTab(current.id);
              }}
            >
              Close
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
