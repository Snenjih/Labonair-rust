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
import { useNotificationStore } from "@/modules/notifications/store/useNotificationStore";
import { git } from "../lib/gitInvoke";
import { useSourceControlStore } from "../store/sourceControlStore";

/**
 * Mounted once at the app-shell level (not inside BranchBar) so a Force Push
 * can always be confirmed regardless of which sidebar panel is active —
 * e.g. when triggered from the Command Palette while Source Control isn't
 * the visible panel.
 */
export function ForcePushConfirmDialog() {
  const open = useSourceControlStore((s) => s.forcePushConfirmOpen);
  const close = useSourceControlStore((s) => s.closeForcePushConfirm);
  const repoRoot = useSourceControlStore((s) => s.repoRoot);
  const sessionId = useSourceControlStore((s) => s.sessionId);
  const currentBranch = useSourceControlStore((s) => s.currentBranch);
  const operationInProgress = useSourceControlStore((s) => s.operationInProgress);
  const setOperationInProgress = useSourceControlStore((s) => s.setOperationInProgress);

  async function handleForcePush() {
    close();
    if (!repoRoot || operationInProgress) return;
    setOperationInProgress("push");
    try {
      await git.pushForceWithLease(repoRoot, undefined, undefined, sessionId ?? undefined);
      useNotificationStore.getState().addActionResultNotification({
        type: "success",
        title: "Force Pushed",
        message: currentBranch ? `${currentBranch} force-pushed to remote` : "Force-pushed to remote",
      });
    } catch (e) {
      useNotificationStore
        .getState()
        .addActionResultNotification({ type: "error", title: "Force Push Failed", message: String(e) });
    } finally {
      setOperationInProgress(null);
    }
  }

  return (
    <AlertDialog open={open} onOpenChange={(v) => (v ? undefined : close())}>
      <AlertDialogContent size="sm">
        <AlertDialogHeader>
          <AlertDialogTitle>Force Push?</AlertDialogTitle>
          <AlertDialogDescription>
            This will overwrite the remote branch. Force-with-lease protects against overwriting others' work
            if they pushed after your last fetch — but it is still a destructive operation.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={() => void handleForcePush()}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
          >
            Force Push
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
