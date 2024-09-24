/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import type {FieldConfig} from './types';
import type {ReactNode, RefObject} from 'react';

import {
  useUploadFilesCallback,
  ImageDropZone,
  FilePicker,
  PendingImageUploads,
} from '../ImageUpload';
import {Internal} from '../Internal';
import {insertAtCursor} from '../textareaUtils';
import {GenerateAIButton} from './GenerateWithAI';
import {MinHeightTextField} from './MinHeightTextField';
import {convertFieldNameToKey} from './utils';
import {TextArea} from 'isl-components/TextArea';
import {useRef, useEffect} from 'react';

function moveCursorToEnd(element: HTMLTextAreaElement) {
  element.setSelectionRange(element.value.length, element.value.length);
}

export function CommitInfoTextArea({
  kind,
  fieldConfig,
  name,
  autoFocus,
  editedMessage,
  setEditedCommitMessage,
}: {
  kind: 'title' | 'textarea' | 'field';
  fieldConfig: FieldConfig;
  name: string;
  autoFocus: boolean;
  editedMessage: string;
  setEditedCommitMessage: (fieldValue: string) => unknown;
}) {
  const ref = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    if (ref.current && autoFocus) {
      ref.current.focus();
      moveCursorToEnd(ref.current);
    }
  }, [autoFocus, ref]);
  const Component = kind === 'field' || kind === 'title' ? MinHeightTextField : TextArea;
  const props =
    kind === 'field' || kind === 'title'
      ? {}
      : ({
          rows: 15,
          resize: 'vertical',
        } as const);

  // The gh cli does not support uploading images for commit messages,
  // see https://github.com/cli/cli/issues/1895#issuecomment-718899617
  // for now, this is internal-only.
  const supportsImageUpload =
    kind === 'textarea' &&
    (Internal.supportsImageUpload === true ||
      // image upload is always enabled in tests
      process.env.NODE_ENV === 'test');

  const onInput = (event: {currentTarget: HTMLTextAreaElement}) => {
    setEditedCommitMessage(event.currentTarget?.value);
  };

  const uploadFiles = useUploadFilesCallback(name, ref, onInput);

  const fieldKey = convertFieldNameToKey(name);

  const rendered = (
    <div className="commit-info-field">
      <Component
        ref={ref}
        {...props}
        onPaste={
          !supportsImageUpload
            ? undefined
            : (event: React.ClipboardEvent<HTMLTextAreaElement>) => {
                if (event.clipboardData != null && event.clipboardData.files.length > 0) {
                  uploadFiles([...event.clipboardData.files]);
                  event.preventDefault();
                }
              }
        }
        value={editedMessage}
        data-testid={`commit-info-${fieldKey}-field`}
        onInput={onInput}
      />
      <EditorToolbar
        fieldName={name}
        uploadFiles={supportsImageUpload ? uploadFiles : undefined}
        supportsGeneratingAIMessage={
          fieldConfig.supportsBeingAutoGenerated && Internal.generateAICommitMessage
        }
        appendToTextArea={(toAdd: string) => {
          const textarea = ref.current;
          if (textarea) {
            insertAtCursor(textarea, toAdd);
            onInput({currentTarget: textarea});
          }
        }}
        textAreaRef={ref}
      />
    </div>
  );
  return !supportsImageUpload ? (
    rendered
  ) : (
    <ImageDropZone onDrop={uploadFiles}>{rendered}</ImageDropZone>
  );
}

/**
 * Floating button list at the bottom corner of the text area
 */
export function EditorToolbar({
  fieldName,
  textAreaRef,
  uploadFiles,
  appendToTextArea,
  supportsGeneratingAIMessage,
}: {
  fieldName: string;
  uploadFiles?: (files: Array<File>) => unknown;
  textAreaRef: RefObject<HTMLTextAreaElement>;
  appendToTextArea: (toAdd: string) => unknown;
  supportsGeneratingAIMessage?: unknown;
}) {
  const parts: Array<ReactNode> = [];
  if (uploadFiles != null) {
    parts.push(
      <PendingImageUploads fieldName={fieldName} key="pending-uploads" textAreaRef={textAreaRef} />,
    );
    parts.push(<FilePicker key="picker" uploadFiles={uploadFiles} />);
  }
  if (supportsGeneratingAIMessage != null) {
    parts.push(
      <GenerateAIButton
        textAreaRef={textAreaRef}
        appendToTextArea={appendToTextArea}
        fieldName={fieldName}
        key="gen-ai-message"
      />,
    );
  }
  if (parts.length === 0) {
    return null;
  }
  return <div className="text-area-toolbar">{parts}</div>;
}
