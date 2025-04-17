import streamlit as st
import requests
import json

# --- Configuration ---
OLLAMA_BASE_URL = "http://127.0.0.1:11434" # Default Ollama API endpoint

# --- Helper Functions ---

def get_available_models(base_url):
    """Fetches the list of locally available Ollama models."""
    try:
        response = requests.get(f"{base_url}/api/tags")
        response.raise_for_status() # Raise an exception for bad status codes (4xx or 5xx)
        models = response.json().get('models', [])
        # Extract just the model names (e.g., 'llama3:latest')
        return [model['name'] for model in models]
    except requests.exceptions.RequestException as e:
        st.error(f"Error connecting to Ollama API: {e}")
        st.error(f"Is the Ollama server running at {base_url}?")
        return []
    except json.JSONDecodeError:
        st.error("Error decoding the response from Ollama API. Response was:")
        st.error(response.text)
        return []

def generate_ollama_response(base_url, model_name, prompt):
    """Sends a prompt to the Ollama API and yields the response stream."""
    try:
        payload = {
            "model": model_name,
            "prompt": prompt,
            "stream": True  # Use streaming for better UI experience
        }
        response = requests.post(f"{base_url}/api/generate", json=payload, stream=True)
        response.raise_for_status()

        full_response = ""
        # Use st.empty() to create a placeholder that can be updated
        response_placeholder = st.empty()

        for line in response.iter_lines():
            if line:
                try:
                    # Each line is a JSON object, decode it
                    chunk = json.loads(line.decode('utf-8'))
                    # Extract the response part from the chunk
                    response_part = chunk.get('response', '')
                    if response_part:
                        full_response += response_part
                        # Update the placeholder with the accumulating response
                        # Use markdown for potential formatting
                        response_placeholder.markdown(full_response)

                    # Check if the generation is done
                    if chunk.get('done', False):
                        break
                except json.JSONDecodeError:
                    st.warning(f"Skipping invalid JSON line from stream: {line}")
                except Exception as e:
                    st.error(f"Error processing stream chunk: {e}")
                    break
        # Final update after stream ends (optional, usually covered by last chunk)
        response_placeholder.markdown(full_response)
        return full_response # Return the full text if needed elsewhere

    except requests.exceptions.RequestException as e:
        st.error(f"Error during Ollama generation request: {e}")
        return None
    except Exception as e:
        st.error(f"An unexpected error occurred: {e}")
        return None


# --- Streamlit App UI ---

st.set_page_config(layout="wide") # Use wider layout for better display
st.title("🤖 Ollama Chat Interface")
st.markdown("Interact with your local Ollama models.")

# --- Check Ollama Server Status (Basic Check) ---
try:
    status_check = requests.get(OLLAMA_BASE_URL)
    if status_check.status_code == 200:
         st.success(f"Successfully connected to Ollama server at {OLLAMA_BASE_URL}")
    else:
         st.warning(f"Ollama server responded with status {status_check.status_code}. Ensure it's running correctly.")
except requests.exceptions.ConnectionError:
     st.error(f"❌ Connection Error: Could not connect to Ollama server at {OLLAMA_BASE_URL}.")
     st.info("Please ensure the 'ollama serve' command is running in a separate terminal.")
     st.stop() # Stop the Streamlit app if connection fails


# --- Model Selection ---
available_models = get_available_models(OLLAMA_BASE_URL)

if not available_models:
    st.warning("No models found from Ollama API. Have you pulled any models? (e.g., `ollama pull llama3`)")
    selected_model = None
else:
    selected_model = st.selectbox(
        "Select an Ollama Model:",
        available_models,
        index=0, # Default to the first model in the list
        help="Choose the model you want to chat with."
    )

# --- Chat Input and Output ---
st.markdown("---") # Separator

# Use session state to keep track of chat history (optional but good for context)
if 'messages' not in st.session_state:
    st.session_state.messages = []

# Display previous messages
for message in st.session_state.messages:
    with st.chat_message(message["role"]):
        st.markdown(message["content"])

# Get user input
prompt = st.chat_input("What would you like to ask?")

if prompt and selected_model:
    # Add user message to chat history
    st.session_state.messages.append({"role": "user", "content": prompt})
    # Display user message
    with st.chat_message("user"):
        st.markdown(prompt)

    # Display assistant response (streamed)
    with st.chat_message("assistant"):
        st.markdown(f"**Asking {selected_model}...**") # Indicate which model is used
        # Use the streaming generation function
        full_bot_response = generate_ollama_response(OLLAMA_BASE_URL, selected_model, prompt)

        if full_bot_response:
             # Add bot response to chat history after generation is complete
             st.session_state.messages.append({"role": "assistant", "content": full_bot_response})
        else:
             st.error("Failed to get a response from the model.")

elif prompt and not selected_model:
    st.warning("Please select a model from the dropdown above first.")

# --- Footer/Info ---
st.markdown("---")
st.markdown(f"Connected to Ollama at `{OLLAMA_BASE_URL}`")
if available_models:
    st.markdown(f"Available models: {', '.join(available_models)}")
