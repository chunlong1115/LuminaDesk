import { useState } from "react";
import { setActiveIfWidgets } from "../../lib/windowRole";
import { isValidTodoText, useTodos } from "../../stores/todo";
import styles from "./widgets.module.css";

/** 待办组件：输入添加、勾选完成、逐项删除 */
export function Todo() {
  const todos = useTodos((s) => s.todos);
  const add = useTodos((s) => s.add);
  const toggle = useTodos((s) => s.toggle);
  const remove = useTodos((s) => s.remove);
  const [text, setText] = useState("");

  const submit = () => {
    if (!isValidTodoText(text)) return;
    add(text);
    setText("");
  };

  return (
    <div className={styles.todo}>
      <div className={styles.todoInputRow}>
        <input
          className={styles.todoInput}
          value={text}
          placeholder="添加待办…"
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") submit();
          }}
          onFocus={() => setActiveIfWidgets(window.location.search, true)}
          onBlur={() => setActiveIfWidgets(window.location.search, false)}
        />
        <button className={styles.todoAdd} onClick={submit} title="添加">
          ＋
        </button>
      </div>
      <ul className={styles.todoList}>
        {todos.map((t) => (
          <li key={t.id} className={styles.todoItem}>
            <label className={styles.todoLabel}>
              <input
                type="checkbox"
                checked={t.done}
                onChange={() => toggle(t.id)}
              />
              <span className={t.done ? styles.todoDone : undefined}>{t.text}</span>
            </label>
            <button
              className={styles.todoRemove}
              title="删除"
              onClick={() => remove(t.id)}
            >
              ×
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
