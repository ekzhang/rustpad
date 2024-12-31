import {
  AlertDialog,
  AlertDialogBody,
  AlertDialogContent,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogOverlay,
  Button,
} from "@chakra-ui/react";
import { useRef } from "react";

export type ReadCodeConfirmProps = {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void;
};

/** Dialog for the "read the code" button when it clears the editor. */
function ReadCodeConfirm({ isOpen, onClose, onConfirm }: ReadCodeConfirmProps) {
  const cancelRef = useRef<HTMLButtonElement>(null);

  return (
    <AlertDialog
      isOpen={isOpen}
      leastDestructiveRef={cancelRef}
      onClose={onClose}
    >
      <AlertDialogOverlay>
        <AlertDialogContent>
          <AlertDialogHeader>Clear editor</AlertDialogHeader>

          <AlertDialogBody>
            Opening Rustpad's source code will clear the existing shared
            content. Is this okay?
          </AlertDialogBody>

          <AlertDialogFooter>
            <Button ref={cancelRef} onClick={onClose}>
              Cancel
            </Button>
            <Button colorScheme="red" onClick={onConfirm} ml={3}>
              Clear
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialogOverlay>
    </AlertDialog>
  );
}

export default ReadCodeConfirm;
